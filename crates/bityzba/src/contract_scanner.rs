/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Spacing, Span, TokenStream, TokenTree};
use syn::{
    Attribute, Fields, File, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct,
    ItemTrait, TraitItem, TraitItemFn,
};
use walkdir::WalkDir;

/// Source scanner that enforces explicit bityzba contract decisions.
#[derive(Debug)]
pub struct ContractScanner {
    manifest_dir: PathBuf,
}

impl ContractScanner {
    /// Create a scanner for a Cargo package directory.
    pub fn new(manifest_dir: impl Into<PathBuf>) -> Self {
        Self {
            manifest_dir: manifest_dir.into(),
        }
    }

    /// Create a scanner for the package currently being built by Cargo.
    pub fn from_cargo_env() -> Result<Self, ContractScanError> {
        let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| ContractScanError::setup("CARGO_MANIFEST_DIR is not set"))?;
        Ok(Self::new(manifest_dir))
    }

    /// Scan the package and return all missing-contract diagnostics at once.
    pub fn scan(&self) -> Result<(), ContractScanError> {
        self.scan_inner(false)
    }

    fn scan_for_build_script(&self) -> Result<(), ContractScanError> {
        self.scan_inner(true)
    }

    fn scan_inner(&self, emit_rerun_if_changed: bool) -> Result<(), ContractScanError> {
        let files = self.rust_files()?;
        let mut diagnostics = Vec::new();

        for path in files {
            if emit_rerun_if_changed {
                println!("cargo:rerun-if-changed={}", path.display());
            }
            let contents = fs::read_to_string(&path).map_err(|error| {
                ContractScanError::setup(format!("failed to read {}: {error}", path.display()))
            })?;
            let display_path = display_path(&self.manifest_dir, &path);
            match syn::parse_file(&contents) {
                Ok(file) => {
                    let mut scanner = FileScanner::new(display_path);
                    scanner.scan_file(&file);
                    diagnostics.extend(scanner.diagnostics);
                }
                Err(error) => diagnostics.push(Diagnostic::new(
                    display_path,
                    error.span().start().line,
                    format!("failed to parse Rust source for bityzba contract scan: {error}"),
                    "fix the parse error before contract scanning can continue",
                )),
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(ContractScanError::diagnostics(diagnostics))
        }
    }

    fn rust_files(&self) -> Result<Vec<PathBuf>, ContractScanError> {
        let mut files = Vec::new();

        for root in ["src", "tests", "benches", "examples"] {
            let root = self.manifest_dir.join(root);
            if !root.exists() {
                continue;
            }
            for entry in WalkDir::new(&root) {
                let entry = entry.map_err(|error| {
                    ContractScanError::setup(format!("failed to walk {}: {error}", root.display()))
                })?;
                if entry.file_type().is_file()
                    && entry.path().extension().is_some_and(|ext| ext == "rs")
                {
                    files.push(entry.path().to_owned());
                }
            }
        }

        let build_script = self.manifest_dir.join("build.rs");
        if build_script.is_file() {
            files.push(build_script);
        }

        files.sort();
        files.dedup();
        Ok(files)
    }
}

/// Scan the current Cargo package and fail the build when explicit bityzba
/// contract markers are missing.
pub fn require_contracts() -> Result<(), ContractScanError> {
    ContractScanner::from_cargo_env()?.scan_for_build_script()
}

/// Error returned when contract scanning fails.
pub struct ContractScanError {
    kind: ContractScanErrorKind,
}

enum ContractScanErrorKind {
    Setup(String),
    Diagnostics(Vec<Diagnostic>),
}

impl ContractScanError {
    fn setup(message: impl Into<String>) -> Self {
        Self {
            kind: ContractScanErrorKind::Setup(message.into()),
        }
    }

    fn diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            kind: ContractScanErrorKind::Diagnostics(diagnostics),
        }
    }
}

impl fmt::Display for ContractScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ContractScanErrorKind::Setup(message) => f.write_str(message),
            ContractScanErrorKind::Diagnostics(diagnostics) => {
                for (index, diagnostic) in diagnostics.iter().enumerate() {
                    if index > 0 {
                        writeln!(f)?;
                    }
                    writeln!(
                        f,
                        "{}:{}: {}",
                        diagnostic.path, diagnostic.line, diagnostic.message
                    )?;
                    write!(f, "help: {}", diagnostic.help)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for ContractScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for ContractScanError {}

struct Diagnostic {
    path: String,
    line: usize,
    message: String,
    help: &'static str,
}

impl Diagnostic {
    fn new(path: String, line: usize, message: impl Into<String>, help: &'static str) -> Self {
        Self {
            path,
            line,
            message: message.into(),
            help,
        }
    }
}

struct FileScanner {
    path: String,
    diagnostics: Vec<Diagnostic>,
}

impl FileScanner {
    fn new(path: String) -> Self {
        Self {
            path,
            diagnostics: Vec::new(),
        }
    }

    fn scan_file(&mut self, file: &File) {
        self.scan_items(&file.items);
    }

    fn scan_items(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Fn(item) => self.scan_free_function(item),
                Item::Struct(item) => self.scan_struct(item),
                Item::Enum(item) => self.scan_enum(item),
                Item::Trait(item) => self.scan_trait(item),
                Item::Impl(item) => self.scan_impl(item),
                Item::Mod(item) => self.scan_mod(item),
                _ => {}
            }
        }
    }

    fn scan_mod(&mut self, item: &ItemMod) {
        if let Some((_brace, items)) = &item.content {
            self.scan_items(items);
        }
    }

    fn scan_free_function(&mut self, item: &ItemFn) {
        self.require_function_contracts(
            &item.attrs,
            "function",
            &item.sig.ident.to_string(),
            item.sig.ident.span(),
        );
    }

    fn scan_struct(&mut self, item: &ItemStruct) {
        if !has_type_invariant(&item.attrs) {
            self.diagnostics.push(Diagnostic::new(
                self.path.clone(),
                item.ident.span().start().line,
                format!("missing bityzba type invariant on struct `{}`", item.ident),
                "add `#[invariant(...)]`; reason carefully about what the type invariant must be, and only use `#[invariant(true)]` when the field types already express the invariant",
            ));
        }
    }

    fn scan_enum(&mut self, item: &ItemEnum) {
        let has_data_variants = item
            .variants
            .iter()
            .any(|variant| !matches!(variant.fields, Fields::Unit));

        if has_data_variants && !has_type_invariant(&item.attrs) {
            self.diagnostics.push(Diagnostic::new(
                self.path.clone(),
                item.ident.span().start().line,
                format!("missing bityzba type invariant on enum `{}`", item.ident),
                "add `#[invariant(...)]`; reason carefully about what the type invariant must be, and only use `#[invariant(true)]` when the variant data already expresses the invariant",
            ));
        }

        let variant_invariants = enum_variant_invariants(&item.attrs);
        for variant in &item.variants {
            if matches!(variant.fields, Fields::Unit) {
                continue;
            }
            let variant_name = variant.ident.to_string();
            if !variant_invariants.contains(&variant_name) {
                self.diagnostics.push(Diagnostic::new(
                    self.path.clone(),
                    variant.ident.span().start().line,
                    format!(
                        "missing bityzba invariant on data-carrying enum variant `{}::{variant_name}`",
                        item.ident
                    ),
                    "add `#[invariant(::Variant => ...)]`; use `#[invariant(::Variant => true)]` only when the variant data already expresses the invariant",
                ));
            }
        }
    }

    fn scan_trait(&mut self, item: &ItemTrait) {
        if !has_attr_named(&item.attrs, "contract_trait") {
            self.diagnostics.push(Diagnostic::new(
                self.path.clone(),
                item.ident.span().start().line,
                format!("missing bityzba contract_trait on trait `{}`", item.ident),
                "add `#[contract_trait]` and explicit contracts to each trait method",
            ));
        }

        for trait_item in &item.items {
            if let TraitItem::Fn(method) = trait_item {
                self.scan_trait_method(&item.ident.to_string(), method);
            }
        }
    }

    fn scan_trait_method(&mut self, trait_name: &str, method: &TraitItemFn) {
        self.require_function_contracts(
            &method.attrs,
            "trait method",
            &format!("{trait_name}::{}", method.sig.ident),
            method.sig.ident.span(),
        );
    }

    fn scan_impl(&mut self, item: &ItemImpl) {
        if item.trait_.is_some() {
            return;
        }

        for impl_item in &item.items {
            if let ImplItem::Fn(method) = impl_item {
                self.require_function_contracts(
                    &method.attrs,
                    "method",
                    &method.sig.ident.to_string(),
                    method.sig.ident.span(),
                );
            }
        }
    }

    fn require_function_contracts(
        &mut self,
        attrs: &[Attribute],
        item_kind: &str,
        item_name: &str,
        span: Span,
    ) {
        let line = span.start().line;
        if !has_precondition(attrs) {
            self.diagnostics.push(Diagnostic::new(
                self.path.clone(),
                line,
                format!("missing bityzba precondition on {item_kind} `{item_name}`"),
                "add `#[requires(...)]`; reason carefully about what the precondition must be, and only use `#[requires(true)]` as a last resort",
            ));
        }
        if !has_postcondition(attrs) {
            self.diagnostics.push(Diagnostic::new(
                self.path.clone(),
                line,
                format!("missing bityzba postcondition on {item_kind} `{item_name}`"),
                "add `#[ensures(...)]`; reason carefully about what the postcondition must be, and only use `#[ensures(true)]` as a last resort",
            ));
        }
    }
}

fn has_precondition(attrs: &[Attribute]) -> bool {
    has_any_attr_named(attrs, &["requires", "expensive_requires"])
}

fn has_postcondition(attrs: &[Attribute]) -> bool {
    has_any_attr_named(attrs, &["ensures", "expensive_ensures"])
}

fn has_type_invariant(attrs: &[Attribute]) -> bool {
    has_any_attr_named(attrs, &["invariant", "expensive_invariant"])
}

fn has_any_attr_named(attrs: &[Attribute], names: &[&str]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| names.iter().any(|name| segment.ident == *name))
    })
}

fn has_attr_named(attrs: &[Attribute], name: &str) -> bool {
    has_any_attr_named(attrs, &[name])
}

fn enum_variant_invariants(attrs: &[Attribute]) -> BTreeSet<String> {
    let mut variants = BTreeSet::new();
    for attr in attrs {
        if !has_any_attr_named(
            std::slice::from_ref(attr),
            &["invariant", "expensive_invariant"],
        ) {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        for segment in attribute_segments(list.tokens.clone()) {
            if let Some(variant) = variant_invariant_name(segment) {
                variants.insert(variant);
            }
        }
    }
    variants
}

fn attribute_segments(tokens: TokenStream) -> Vec<TokenStream> {
    let mut segments = Vec::new();
    let mut segment = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Punct(punct)
                if punct.as_char() == ',' && punct.spacing() == Spacing::Alone =>
            {
                if !segment.is_empty() {
                    segments.push(segment.into_iter().collect());
                    segment = Vec::new();
                }
            }
            token => segment.push(token),
        }
    }
    if !segment.is_empty() {
        segments.push(segment.into_iter().collect());
    }
    segments
}

fn variant_invariant_name(segment: TokenStream) -> Option<String> {
    let tokens = segment.into_iter().collect::<Vec<_>>();
    if !starts_with_double_colon(&tokens) || top_level_fat_arrow_index(&tokens).is_none() {
        return None;
    }
    match tokens.get(2) {
        Some(TokenTree::Ident(ident)) => Some(ident.to_string()),
        _ => None,
    }
}

fn starts_with_double_colon(tokens: &[TokenTree]) -> bool {
    matches!(
        (tokens.first(), tokens.get(1)),
        (Some(TokenTree::Punct(first)), Some(TokenTree::Punct(second)))
            if first.as_char() == ':'
                && first.spacing() == Spacing::Joint
                && second.as_char() == ':'
    )
}

fn top_level_fat_arrow_index(tokens: &[TokenTree]) -> Option<usize> {
    tokens.windows(2).position(|window| {
        matches!(
            (&window[0], &window[1]),
            (TokenTree::Punct(first), TokenTree::Punct(second))
                if first.as_char() == '='
                    && first.spacing() == Spacing::Joint
                    && second.as_char() == '>'
        )
    })
}

fn display_path(manifest_dir: &Path, path: &Path) -> String {
    let path = path.strip_prefix(manifest_dir).unwrap_or(path);
    path.to_string_lossy().replace('\\', "/")
}

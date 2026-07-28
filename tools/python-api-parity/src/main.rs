//! Mechanical in-scope Rust API inventory and Python parity-matrix checker.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use bityzba::{invariant, new, requires, try_new};
use jbotci_python_syntax_macros::generate_syntax_parity_inventory;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;
use sha2::{Digest, Sha256};
use syn::{
    Attribute, Fields, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMacro, ItemMod, ItemStruct,
    ItemTrait, TraitItem, UseTree, Visibility,
};
use walkdir::WalkDir;

jbotci_syntax::__jbotci_syntax_binding_schema!(generate_syntax_parity_inventory);

const MATRIX_HEADER: &str =
    "rust_path\tkind\trust_signature\tdisposition\tpython_path\texclusion_kind\trationale";
const DISPOSITIONS: &[&str] = &["direct", "python-equivalent", "subsumed", "rust-only"];
const EXCLUSION_KINDS: &[&str] = &[
    "generic-trait",
    "ownership-lifetime",
    "construction-validation",
    "serialization-import",
    "implementation-representation",
    "non-python-platform",
];

#[invariant(!rust_path.is_empty())]
#[invariant(!kind.is_empty())]
#[invariant(!rust_signature.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ApiItem {
    rust_path: String,
    kind: String,
    rust_signature: String,
    suggested_python: Option<String>,
}

impl ApiItem {
    #[requires(!rust_path.is_empty() && !kind.is_empty() && !rust_signature.is_empty())]
    #[ensures(!ret.rust_path.is_empty())]
    fn new(
        rust_path: String,
        kind: &str,
        rust_signature: String,
        suggested_python: Option<String>,
    ) -> Self {
        new!(ApiItem {
            rust_path,
            kind: kind.to_owned(),
            rust_signature,
            suggested_python,
        })
    }
}

#[invariant(!rust_path.is_empty())]
#[invariant(!kind.is_empty())]
#[invariant(!rust_signature.is_empty())]
#[invariant(DISPOSITIONS.contains(&disposition.as_str()))]
#[invariant(disposition == "rust-only" -> python_path.is_empty())]
#[invariant(disposition != "rust-only" -> !python_path.is_empty())]
#[invariant(disposition == "rust-only" -> EXCLUSION_KINDS.contains(&exclusion_kind.as_str()))]
#[invariant(disposition != "rust-only" -> exclusion_kind.is_empty())]
#[invariant(disposition == "rust-only" -> !rationale.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct MatrixEntry {
    rust_path: String,
    kind: String,
    rust_signature: String,
    disposition: String,
    python_path: String,
    exclusion_kind: String,
    rationale: String,
}

#[invariant(!crate_name.is_empty())]
#[invariant(!source_dir.as_os_str().is_empty())]
#[derive(Debug, Clone)]
struct CrateScope {
    crate_name: &'static str,
    source_dir: PathBuf,
    selected_files: Option<BTreeSet<PathBuf>>,
}

#[invariant(!crate_name.is_empty())]
#[derive(Debug)]
struct SourceContext {
    crate_name: String,
    module_path: Vec<String>,
}

#[invariant(!name.is_empty())]
#[invariant(!variants.is_empty())]
#[derive(Debug)]
struct SyntheticEnum {
    name: String,
    variants: Vec<String>,
    members: Vec<SyntheticMember>,
}

#[invariant(!name.is_empty())]
#[invariant(!kind.is_empty())]
#[invariant(!signature.is_empty())]
#[derive(Debug)]
struct SyntheticMember {
    name: String,
    kind: String,
    signature: String,
}

#[requires(true)]
#[ensures(true)]
fn main() {
    if let Err(message) = run(env::args().skip(1).collect()) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn run(arguments: Vec<String>) -> Result<(), String> {
    let workspace = workspace_root()?;
    match arguments.as_slice() {
        [command] if command == "inventory" => {
            let inventory = rust_inventory(&workspace)?;
            print_inventory(&inventory);
            Ok(())
        }
        [command, matrix] if command == "check" => {
            let inventory = rust_inventory(&workspace)?;
            check_matrix(&inventory, &workspace.join(matrix))
        }
        [command, matrix] if command == "template" => {
            let inventory = rust_inventory(&workspace)?;
            write_template(&inventory, &workspace.join(matrix))
        }
        [command, source] if command == "pyclass-docs" => {
            print_pyclass_docs(&workspace.join(source))
        }
        _ => Err(
            "usage: jbotci-python-api-parity inventory | check <matrix.tsv> | template \
             <matrix.tsv> | pyclass-docs <source.rs>"
                .to_owned(),
        ),
    }
}

#[requires(source_path.is_absolute())]
#[ensures(ret.is_ok() || ret.is_err())]
fn print_pyclass_docs(source_path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("failed to read {}: {error}", source_path.display()))?;
    let parsed = syn::parse_file(&source)
        .map_err(|error| format!("failed to parse {}: {error}", source_path.display()))?;
    for item in parsed.items {
        let Item::Struct(item) = item else {
            continue;
        };
        let Some(python_name) = pyclass_name(&item.attrs)? else {
            continue;
        };
        let documentation = rust_documentation(&item.attrs);
        if documentation.is_empty() {
            continue;
        }
        println!("{python_name}\t{}", hex_encode(documentation.as_bytes()));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn pyclass_name(attributes: &[Attribute]) -> Result<Option<String>, String> {
    let Some(attribute) = attributes
        .iter()
        .find(|attribute| attribute.path().is_ident("pyclass"))
    else {
        return Ok(None);
    };
    let mut name = None;
    attribute
        .parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                name = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            } else if meta.input.peek(syn::Token![=]) {
                let _ = meta.value()?.parse::<syn::Expr>()?;
            }
            Ok(())
        })
        .map_err(|error| format!("invalid #[pyclass] attribute: {error}"))?;
    Ok(name)
}

#[requires(true)]
#[ensures(true)]
fn rust_documentation(attributes: &[Attribute]) -> String {
    attributes
        .iter()
        .filter_map(|attribute| {
            if !attribute.path().is_ident("doc") {
                return None;
            }
            let syn::Meta::NameValue(value) = &attribute.meta else {
                return None;
            };
            let syn::Expr::Lit(value) = &value.value else {
                return None;
            };
            let syn::Lit::Str(value) = &value.lit else {
                return None;
            };
            Some(value.value().trim().to_owned())
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[requires(true)]
#[ensures(ret.len() == bytes.len() * 2)]
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_absolute()) || ret.is_err())]
fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "parity tool manifest is not under the workspace tools directory".to_owned())
}

#[requires(workspace.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|items| !items.is_empty()) || ret.is_err())]
fn rust_inventory(workspace: &Path) -> Result<Vec<ApiItem>, String> {
    let syntax_source = workspace.join("crates/jbotci-syntax/src");
    let syntax_files = ["lib.rs", "tree.rs", "grammar/mod.rs"]
        .into_iter()
        .map(|path| syntax_source.join(path))
        .collect::<BTreeSet<_>>();
    let scopes = [
        scope(workspace, "jbotci_source", "crates/jbotci-source/src", None),
        scope(
            workspace,
            "jbotci_diagnostics",
            "crates/jbotci-diagnostics/src",
            None,
        ),
        scope(
            workspace,
            "jbotci_dialect",
            "crates/jbotci-dialect/src",
            None,
        ),
        scope(
            workspace,
            "jbotci_morphology",
            "crates/jbotci-morphology/src",
            None,
        ),
        scope(
            workspace,
            "jbotci_syntax",
            "crates/jbotci-syntax/src",
            Some(syntax_files),
        ),
        scope(
            workspace,
            "jbotci_dictionary",
            "crates/jbotci-dictionary/src",
            None,
        ),
        scope(
            workspace,
            "jbotci_dictionary_data",
            "crates/jbotci-dictionary-data/src",
            None,
        ),
        scope(workspace, "jbotci_jvozba", "crates/jbotci-jvozba/src", None),
        scope(
            workspace,
            "jbotci_semantics",
            "crates/jbotci-semantics/src",
            Some(BTreeSet::from([
                workspace.join("crates/jbotci-semantics/src/references.rs")
            ])),
        ),
    ];
    let mut items = BTreeSet::new();
    for scope in scopes {
        inventory_scope(&scope, &mut items)?;
    }
    for (rust_path, kind, signature, python_path) in GENERATED_SYNTAX_API {
        items.insert(ApiItem::new(
            (*rust_path).to_owned(),
            kind,
            (*signature).to_owned(),
            Some((*python_path).to_owned()),
        ));
    }
    for (name, path) in [
        (
            "__tree_model_generator_source",
            "crates/jbotci-tree-macros/src/lib.rs",
        ),
        (
            "__syntax_model_generator_source",
            "crates/jbotci-syntax-macros/src/lib.rs",
        ),
    ] {
        items.insert(ApiItem::new(
            format!("jbotci_syntax::generated_model::{name}"),
            "generator-source",
            source_fingerprint(&workspace.join(path))?,
            None,
        ));
    }
    filter_reachable_api(workspace, items)
}

#[requires(path.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|value| value.starts_with("sha256:")) || ret.is_err())]
fn source_fingerprint(path: &Path) -> Result<String, String> {
    let source =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let digest = Sha256::digest(source);
    Ok(format!("sha256:{digest:x}"))
}

#[requires(workspace.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|items| !items.is_empty()) || ret.is_err())]
fn filter_reachable_api(
    workspace: &Path,
    raw_items: BTreeSet<ApiItem>,
) -> Result<Vec<ApiItem>, String> {
    let crate_roots = [
        ("jbotci_source", "crates/jbotci-source/src/lib.rs"),
        ("jbotci_diagnostics", "crates/jbotci-diagnostics/src/lib.rs"),
        ("jbotci_dialect", "crates/jbotci-dialect/src/lib.rs"),
        ("jbotci_morphology", "crates/jbotci-morphology/src/lib.rs"),
        ("jbotci_syntax", "crates/jbotci-syntax/src/lib.rs"),
        ("jbotci_dictionary", "crates/jbotci-dictionary/src/lib.rs"),
        (
            "jbotci_dictionary_data",
            "crates/jbotci-dictionary-data/src/lib.rs",
        ),
        ("jbotci_jvozba", "crates/jbotci-jvozba/src/lib.rs"),
        ("jbotci_semantics", "crates/jbotci-semantics/src/lib.rs"),
    ];
    let mut private_modules = BTreeMap::<String, BTreeSet<String>>::new();
    let mut reexports = Vec::<(String, String)>::new();
    for (crate_name, relative_root) in crate_roots {
        let root_path = workspace.join(relative_root);
        let source = fs::read_to_string(&root_path)
            .map_err(|error| format!("failed to read {}: {error}", root_path.display()))?;
        let parsed = syn::parse_file(&source)
            .map_err(|error| format!("failed to parse {}: {error}", root_path.display()))?;
        for item in parsed.items {
            match item {
                Item::Mod(module) if !is_public(&module.vis) => {
                    private_modules
                        .entry(crate_name.to_owned())
                        .or_default()
                        .insert(module.ident.to_string());
                }
                Item::Use(value) if is_public(&value.vis) => {
                    let mut leaves = Vec::new();
                    flatten_use(&value.tree, Vec::new(), &mut leaves);
                    for (source, alias) in leaves {
                        let Some((source, target)) =
                            reexport_prefix(crate_name, &source, alias.as_deref())
                        else {
                            continue;
                        };
                        reexports.push((source, target));
                    }
                }
                _ => {}
            }
        }
    }

    let mut reachable = BTreeSet::new();
    for item in raw_items {
        if item.kind == "reexport" {
            continue;
        }
        let mut components = item.rust_path.split("::");
        let Some(crate_name) = components.next() else {
            continue;
        };
        let Some(first_item) = components.next() else {
            continue;
        };
        let declaration_is_directly_reachable = !private_modules
            .get(crate_name)
            .is_some_and(|modules| modules.contains(first_item));
        if declaration_is_directly_reachable {
            reachable.insert(item);
            continue;
        }
        let Some((source, target)) = reexports.iter().find(|(source, target)| {
            source.starts_with(&format!("{crate_name}::"))
                && target.starts_with(&format!("{crate_name}::"))
                && path_is_or_descends_from(&item.rust_path, source)
        }) else {
            continue;
        };
        let suffix = &item.rust_path[source.len()..];
        let public_alias = format!("{target}{suffix}");
        let item = item.into_data();
        let kind = item.kind;
        reachable.insert(ApiItem::new(
            item.rust_path,
            &kind,
            item.rust_signature,
            python_path_for_rust_path(&public_alias).or(item.suggested_python),
        ));
    }
    Ok(reachable.into_iter().collect())
}

#[requires(!crate_name.is_empty() && !source.is_empty())]
#[ensures(true)]
fn reexport_prefix(
    crate_name: &str,
    source: &[String],
    alias: Option<&str>,
) -> Option<(String, String)> {
    let (source_crate, source_components) = match source {
        [first, rest @ ..] if first == "crate" => (crate_name, rest),
        [first, rest @ ..] if first.starts_with("jbotci_") => (first.as_str(), rest),
        _ => (crate_name, source),
    };
    let (source_name, source_module) = source_components.split_last()?;
    let is_glob = source_name == "*";
    let source_suffix = if is_glob {
        source_module
    } else {
        source_components
    };
    let mut source_path = source_crate.to_owned();
    for component in source_suffix {
        write!(source_path, "::{component}").expect("writing to String cannot fail");
    }

    let target_name = if is_glob {
        None
    } else {
        Some(alias.unwrap_or(source_name))
    };
    let mut target_path = crate_name.to_owned();
    if let Some(target_name) = target_name {
        write!(target_path, "::{target_name}").expect("writing to String cannot fail");
    }
    Some((source_path, target_path))
}

#[requires(!path.is_empty() && !prefix.is_empty())]
#[ensures(true)]
fn path_is_or_descends_from(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with("::"))
}

#[requires(!rust_path.is_empty())]
#[ensures(true)]
fn python_path_for_rust_path(rust_path: &str) -> Option<String> {
    let (crate_name, suffix) = rust_path.split_once("::")?;
    let module = match crate_name {
        "jbotci_source" => "jbotci.source",
        "jbotci_diagnostics" => "jbotci.diagnostics",
        "jbotci_dialect" => "jbotci.dialect",
        "jbotci_morphology" => "jbotci.morphology",
        "jbotci_syntax" => "jbotci.syntax",
        "jbotci_dictionary" | "jbotci_dictionary_data" => "jbotci.dictionary",
        "jbotci_jvozba" => "jbotci.jvozba",
        "jbotci_semantics" => "jbotci.semantics.references",
        _ => return None,
    };
    Some(format!("{module}.{}", suffix.replace("::", ".")))
}

#[requires(workspace.is_absolute() && !crate_name.is_empty() && !relative_source.is_empty())]
#[ensures(ret.crate_name == crate_name)]
fn scope(
    workspace: &Path,
    crate_name: &'static str,
    relative_source: &str,
    selected_files: Option<BTreeSet<PathBuf>>,
) -> CrateScope {
    new!(CrateScope {
        crate_name,
        source_dir: workspace.join(relative_source),
        selected_files,
    })
}

#[requires(!scope.crate_name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn inventory_scope(scope: &CrateScope, items: &mut BTreeSet<ApiItem>) -> Result<(), String> {
    let mut source_files = WalkDir::new(&scope.source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .filter(|path| {
            scope
                .selected_files
                .as_ref()
                .is_none_or(|selected| selected.contains(path))
        })
        .collect::<Vec<_>>();
    source_files.sort();
    for source_file in source_files {
        let source = fs::read_to_string(&source_file)
            .map_err(|error| format!("failed to read {}: {error}", source_file.display()))?;
        let parsed = syn::parse_file(&source)
            .map_err(|error| format!("failed to parse {}: {error}", source_file.display()))?;
        let module_path = module_path_for_file(&scope.source_dir, &source_file)?;
        let context = new!(SourceContext {
            crate_name: scope.crate_name.to_owned(),
            module_path,
        });
        inventory_items(&context, &parsed.items, items)?;
    }
    Ok(())
}

#[requires(source_dir.is_absolute() && source_file.is_absolute())]
#[ensures(ret.is_ok() || ret.is_err())]
fn module_path_for_file(source_dir: &Path, source_file: &Path) -> Result<Vec<String>, String> {
    let relative = source_file.strip_prefix(source_dir).map_err(|_| {
        format!(
            "{} is outside {}",
            source_file.display(),
            source_dir.display()
        )
    })?;
    let mut parts = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let filename = parts
        .pop()
        .ok_or_else(|| format!("{} has no filename", source_file.display()))?;
    match filename.as_str() {
        "lib.rs" | "mod.rs" => {}
        _ => {
            let stem = filename
                .strip_suffix(".rs")
                .ok_or_else(|| format!("{filename} is not a Rust source file"))?;
            parts.push(stem.to_owned());
        }
    }
    Ok(parts)
}

#[requires(!context.crate_name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn inventory_items(
    context: &SourceContext,
    source_items: &[Item],
    items: &mut BTreeSet<ApiItem>,
) -> Result<(), String> {
    for item in source_items {
        if cfg_test(attributes(item)) {
            continue;
        }
        match item {
            Item::Const(value) if is_public(&value.vis) => {
                push_named_item(
                    context,
                    &value.ident.to_string(),
                    "constant",
                    value.ty.to_token_stream().to_string(),
                    items,
                );
            }
            Item::Enum(value) if is_public(&value.vis) => {
                inventory_enum(context, value, items);
            }
            Item::Fn(value) if is_public(&value.vis) => {
                inventory_function(context, value, items);
            }
            Item::Impl(value) => inventory_impl(context, value, items),
            Item::Macro(value) => inventory_macro(context, value, items)?,
            Item::Mod(value) if is_public(&value.vis) => {
                inventory_module(context, value, items)?;
            }
            Item::Static(value) if is_public(&value.vis) => {
                push_named_item(
                    context,
                    &value.ident.to_string(),
                    "static",
                    value.ty.to_token_stream().to_string(),
                    items,
                );
            }
            Item::Struct(value) if is_public(&value.vis) => {
                inventory_struct(context, value, items);
            }
            Item::Trait(value) if is_public(&value.vis) => {
                inventory_trait(context, value, items);
            }
            Item::Type(value) if is_public(&value.vis) => {
                push_named_item(
                    context,
                    &value.ident.to_string(),
                    "type-alias",
                    value.ty.to_token_stream().to_string(),
                    items,
                );
            }
            Item::Union(value) if is_public(&value.vis) => {
                push_named_item(
                    context,
                    &value.ident.to_string(),
                    "union",
                    value.generics.to_token_stream().to_string(),
                    items,
                );
                for field in &value.fields.named {
                    if is_public(&field.vis)
                        && let Some(name) = &field.ident
                    {
                        push_child_item(
                            context,
                            &value.ident.to_string(),
                            &name.to_string(),
                            "field",
                            field.ty.to_token_stream().to_string(),
                            items,
                        );
                    }
                }
            }
            Item::Use(value) if is_public(&value.vis) => {
                inventory_use(context, &value.tree, items);
            }
            _ => {}
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(value) => &value.attrs,
        Item::Enum(value) => &value.attrs,
        Item::ExternCrate(value) => &value.attrs,
        Item::Fn(value) => &value.attrs,
        Item::ForeignMod(value) => &value.attrs,
        Item::Impl(value) => &value.attrs,
        Item::Macro(value) => &value.attrs,
        Item::Mod(value) => &value.attrs,
        Item::Static(value) => &value.attrs,
        Item::Struct(value) => &value.attrs,
        Item::Trait(value) => &value.attrs,
        Item::TraitAlias(value) => &value.attrs,
        Item::Type(value) => &value.attrs,
        Item::Union(value) => &value.attrs,
        Item::Use(value) => &value.attrs,
        Item::Verbatim(_) => &[],
        _ => &[],
    }
}

#[requires(true)]
#[ensures(true)]
fn cfg_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| match &attribute.meta {
        syn::Meta::List(list) if list.path.is_ident("cfg") => {
            syn::parse2::<syn::Path>(list.tokens.clone()).is_ok_and(|path| path.is_ident("test"))
        }
        _ => false,
    })
}

#[requires(true)]
#[ensures(ret == matches!(visibility, Visibility::Public(_)))]
fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

#[requires(!name.is_empty() && !kind.is_empty())]
#[ensures(true)]
fn push_named_item(
    context: &SourceContext,
    name: &str,
    kind: &str,
    signature: String,
    items: &mut BTreeSet<ApiItem>,
) {
    let rust_path = qualified_path(context, name);
    let signature = nonempty_signature(signature);
    items.insert(ApiItem::new(
        rust_path,
        kind,
        signature,
        suggested_python(context, name),
    ));
}

#[requires(!parent.is_empty() && !name.is_empty() && !kind.is_empty())]
#[ensures(true)]
fn push_child_item(
    context: &SourceContext,
    parent: &str,
    name: &str,
    kind: &str,
    signature: String,
    items: &mut BTreeSet<ApiItem>,
) {
    let rust_path = qualified_path(context, &format!("{parent}::{name}"));
    let signature = nonempty_signature(signature);
    items.insert(ApiItem::new(
        rust_path,
        kind,
        signature,
        suggested_python(context, &format!("{parent}.{name}")),
    ));
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn nonempty_signature(signature: String) -> String {
    if signature.is_empty() {
        "()".to_owned()
    } else {
        signature
    }
}

#[requires(!suffix.is_empty())]
#[ensures(ret.starts_with(&context.crate_name))]
fn qualified_path(context: &SourceContext, suffix: &str) -> String {
    if context.module_path.is_empty() {
        format!("{}::{suffix}", context.crate_name)
    } else {
        format!(
            "{}::{}::{suffix}",
            context.crate_name,
            context.module_path.join("::")
        )
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn suggested_python(context: &SourceContext, name: &str) -> Option<String> {
    let module = match context.crate_name.as_str() {
        "jbotci_source" => "jbotci.source",
        "jbotci_diagnostics" => "jbotci.diagnostics",
        "jbotci_dialect" => "jbotci.dialect",
        "jbotci_morphology" => "jbotci.morphology",
        "jbotci_syntax" => "jbotci.syntax",
        "jbotci_dictionary" | "jbotci_dictionary_data" => "jbotci.dictionary",
        "jbotci_jvozba" => "jbotci.jvozba",
        "jbotci_semantics" => "jbotci.semantics.references",
        _ => return None,
    };
    Some(format!("{module}.{name}"))
}

#[requires(true)]
#[ensures(true)]
fn inventory_struct(context: &SourceContext, value: &ItemStruct, items: &mut BTreeSet<ApiItem>) {
    push_named_item(
        context,
        &value.ident.to_string(),
        "struct",
        value.generics.to_token_stream().to_string(),
        items,
    );
    inventory_fields(
        context,
        &value.ident.to_string(),
        &value.fields,
        false,
        items,
    );
}

#[requires(true)]
#[ensures(true)]
fn inventory_enum(context: &SourceContext, value: &ItemEnum, items: &mut BTreeSet<ApiItem>) {
    let name = value.ident.to_string();
    push_named_item(
        context,
        &name,
        "enum",
        value.generics.to_token_stream().to_string(),
        items,
    );
    for variant in &value.variants {
        let variant_name = variant.ident.to_string();
        push_child_item(
            context,
            &name,
            &variant_name,
            "variant",
            variant.fields.to_token_stream().to_string(),
            items,
        );
        inventory_fields(
            context,
            &format!("{name}::{variant_name}"),
            &variant.fields,
            true,
            items,
        );
    }
}

#[requires(!parent.is_empty())]
#[ensures(true)]
fn inventory_fields(
    context: &SourceContext,
    parent: &str,
    fields: &Fields,
    enum_variant: bool,
    items: &mut BTreeSet<ApiItem>,
) {
    for (index, field) in fields.iter().enumerate() {
        if !enum_variant && matches!(fields, Fields::Named(_)) && !is_public(&field.vis) {
            continue;
        }
        let name = field
            .ident
            .as_ref()
            .map_or_else(|| index.to_string(), ToString::to_string);
        push_child_item(
            context,
            parent,
            &name,
            "field",
            field.ty.to_token_stream().to_string(),
            items,
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn inventory_function(context: &SourceContext, value: &ItemFn, items: &mut BTreeSet<ApiItem>) {
    push_named_item(
        context,
        &value.sig.ident.to_string(),
        "function",
        value.sig.to_token_stream().to_string(),
        items,
    );
}

#[requires(true)]
#[ensures(true)]
fn inventory_impl(context: &SourceContext, value: &ItemImpl, items: &mut BTreeSet<ApiItem>) {
    if value.trait_.is_some() {
        return;
    }
    let Some(type_name) = impl_type_name(&value.self_ty) else {
        return;
    };
    for member in &value.items {
        match member {
            ImplItem::Const(item) if is_public(&item.vis) => push_child_item(
                context,
                &type_name,
                &item.ident.to_string(),
                "associated-constant",
                item.ty.to_token_stream().to_string(),
                items,
            ),
            ImplItem::Fn(item) if is_public(&item.vis) => push_child_item(
                context,
                &type_name,
                &item.sig.ident.to_string(),
                "method",
                item.sig.to_token_stream().to_string(),
                items,
            ),
            ImplItem::Type(item) if is_public(&item.vis) => push_child_item(
                context,
                &type_name,
                &item.ident.to_string(),
                "associated-type",
                item.ty.to_token_stream().to_string(),
                items,
            ),
            _ => {}
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn impl_type_name(value: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = value else {
        return None;
    };
    Some(
        path.path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
    )
}

#[requires(true)]
#[ensures(true)]
fn inventory_trait(context: &SourceContext, value: &ItemTrait, items: &mut BTreeSet<ApiItem>) {
    let name = value.ident.to_string();
    push_named_item(
        context,
        &name,
        "trait",
        value.generics.to_token_stream().to_string(),
        items,
    );
    for member in &value.items {
        match member {
            TraitItem::Const(item) => push_child_item(
                context,
                &name,
                &item.ident.to_string(),
                "associated-constant",
                item.ty.to_token_stream().to_string(),
                items,
            ),
            TraitItem::Fn(item) => push_child_item(
                context,
                &name,
                &item.sig.ident.to_string(),
                "trait-method",
                item.sig.to_token_stream().to_string(),
                items,
            ),
            TraitItem::Type(item) => push_child_item(
                context,
                &name,
                &item.ident.to_string(),
                "associated-type",
                item.generics.to_token_stream().to_string(),
                items,
            ),
            _ => {}
        }
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn inventory_module(
    context: &SourceContext,
    value: &ItemMod,
    items: &mut BTreeSet<ApiItem>,
) -> Result<(), String> {
    let name = value.ident.to_string();
    push_named_item(context, &name, "module", "public module".to_owned(), items);
    if let Some((_, inline_items)) = &value.content {
        let mut nested_path = context.module_path.clone();
        nested_path.push(name);
        let nested = new!(SourceContext {
            crate_name: context.crate_name.clone(),
            module_path: nested_path,
        });
        inventory_items(&nested, inline_items, items)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn inventory_use(context: &SourceContext, tree: &UseTree, items: &mut BTreeSet<ApiItem>) {
    let mut leaves = Vec::new();
    flatten_use(tree, Vec::new(), &mut leaves);
    for (source, alias) in leaves {
        let alias =
            alias.unwrap_or_else(|| source.last().cloned().unwrap_or_else(|| "*".to_owned()));
        push_named_item(context, &alias, "reexport", source.join("::"), items);
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten_use(
    tree: &UseTree,
    prefix: Vec<String>,
    output: &mut Vec<(Vec<String>, Option<String>)>,
) {
    match tree {
        UseTree::Path(path) => {
            let mut next = prefix;
            next.push(path.ident.to_string());
            flatten_use(&path.tree, next, output);
        }
        UseTree::Name(name) => {
            let mut path = prefix;
            path.push(name.ident.to_string());
            output.push((path, None));
        }
        UseTree::Rename(rename) => {
            let mut path = prefix;
            path.push(rename.ident.to_string());
            output.push((path, Some(rename.rename.to_string())));
        }
        UseTree::Glob(_) => {
            let mut path = prefix;
            path.push("*".to_owned());
            output.push((path, Some("*".to_owned())));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use(item, prefix.clone(), output);
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn inventory_macro(
    context: &SourceContext,
    value: &ItemMacro,
    items: &mut BTreeSet<ApiItem>,
) -> Result<(), String> {
    let name = value
        .mac
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default();
    match name.as_str() {
        "tree_model" => {
            let parsed = syn::parse2::<syn::File>(value.mac.tokens.clone()).map_err(|error| {
                format!(
                    "failed to parse tree_model input in {}: {error}",
                    qualified_path(context, "tree_model!")
                )
            })?;
            inventory_items(context, &parsed.items, items)?;
        }
        "define_string_enum" | "define_string_enum_metadata" => {
            if let Some(synthetic) = parse_named_macro_enum(&value.mac.tokens, &name)? {
                inventory_synthetic_enum(context, &synthetic, items);
            }
        }
        "define_dialect_features" => {
            let variants = parse_arrow_variants(&value.mac.tokens);
            let synthetic = new!(SyntheticEnum {
                name: "DialectFeature".to_owned(),
                variants,
                members: vec![
                    synthetic_member("ALL", "associated-constant", "&'static [Self]"),
                    synthetic_member("all", "method", "fn all() -> &'static [Self]"),
                    synthetic_member("name", "method", "fn name(self) -> &'static str"),
                    synthetic_member("atom_name", "method", "fn atom_name(self) -> String"),
                ],
            });
            inventory_synthetic_enum(context, &synthetic, items);
        }
        "define_selmaho" => {
            let variants = parse_arrow_variants(&value.mac.tokens);
            let synthetic = new!(SyntheticEnum {
                name: "Selmaho".to_owned(),
                variants,
                members: vec![
                    synthetic_member("ALL", "associated-constant", "&'static [Self]"),
                    synthetic_member("name", "method", "fn name(self) -> &'static str"),
                    synthetic_member("from_name", "method", "fn from_name(&str) -> Option<Self>"),
                    synthetic_member("contains", "method", "fn contains(self, Cmavo) -> bool"),
                ],
            });
            inventory_synthetic_enum(context, &synthetic, items);
        }
        "cmavo_table"
            if value
                .ident
                .as_ref()
                .is_some_and(|ident| ident == "cmavo_table") =>
        {
            let variants = parse_cmavo_table_variants(&value.mac.tokens);
            if variants.is_empty() {
                return Err("canonical cmavo table did not yield any variants".to_owned());
            }
            let synthetic = new!(SyntheticEnum {
                name: "Cmavo".to_owned(),
                variants,
                members: Vec::new(),
            });
            inventory_synthetic_enum(context, &synthetic, items);
        }
        _ => {}
    }
    Ok(())
}

#[requires(!name.is_empty() && !kind.is_empty() && !signature.is_empty())]
#[ensures(ret.name == name)]
fn synthetic_member(name: &str, kind: &str, signature: &str) -> SyntheticMember {
    new!(SyntheticMember {
        name: name.to_owned(),
        kind: kind.to_owned(),
        signature: signature.to_owned(),
    })
}

#[requires(!value.name.is_empty())]
#[ensures(true)]
fn inventory_synthetic_enum(
    context: &SourceContext,
    value: &SyntheticEnum,
    items: &mut BTreeSet<ApiItem>,
) {
    push_named_item(
        context,
        &value.name,
        "enum",
        "macro-generated fieldless enum".to_owned(),
        items,
    );
    for variant in &value.variants {
        push_child_item(
            context,
            &value.name,
            variant,
            "variant",
            "unit".to_owned(),
            items,
        );
    }
    for member in &value.members {
        push_child_item(
            context,
            &value.name,
            &member.name,
            &member.kind,
            member.signature.clone(),
            items,
        );
    }
}

#[requires(!macro_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| value.as_ref().is_none_or(|item| !item.name.is_empty())) || ret.is_err())]
fn parse_named_macro_enum(
    tokens: &TokenStream,
    macro_name: &str,
) -> Result<Option<SyntheticEnum>, String> {
    let values = tokens.clone().into_iter().collect::<Vec<_>>();
    let Some(enum_index) = values
        .iter()
        .position(|token| matches!(token, TokenTree::Ident(ident) if ident == "enum"))
    else {
        return Ok(None);
    };
    let Some(TokenTree::Ident(name)) = values.get(enum_index + 1) else {
        return Err(format!("{macro_name}! has `enum` without a type name"));
    };
    let Some(TokenTree::Group(group)) = values.iter().skip(enum_index + 2).find(
        |token| matches!(token, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
    ) else {
        return Err(format!("{macro_name}! enum {} has no body", name));
    };
    let members = match macro_name {
        "define_string_enum" => vec![
            synthetic_member("ALL", "associated-constant", "&'static [Self]"),
            synthetic_member("name", "method", "fn name(self) -> &'static str"),
        ],
        "define_string_enum_metadata" => Vec::new(),
        _ => Vec::new(),
    };
    Ok(Some(new!(SyntheticEnum {
        name: name.to_string(),
        variants: parse_arrow_variants(&group.stream()),
        members,
    })))
}

#[requires(true)]
#[ensures(true)]
fn parse_arrow_variants(tokens: &TokenStream) -> Vec<String> {
    let values = tokens.clone().into_iter().collect::<Vec<_>>();
    let mut variants = Vec::new();
    for window in values.windows(3) {
        if let [
            TokenTree::Ident(ident),
            TokenTree::Punct(equal),
            TokenTree::Punct(angle),
        ] = window
            && equal.as_char() == '='
            && angle.as_char() == '>'
        {
            variants.push(ident.to_string());
        }
    }
    variants.sort();
    variants.dedup();
    variants
}

#[requires(true)]
#[ensures(true)]
fn parse_cmavo_table_variants(tokens: &TokenStream) -> Vec<String> {
    let mut variants = BTreeSet::new();
    collect_cmavo_variants(tokens, &mut variants);
    variants.into_iter().collect()
}

#[requires(true)]
#[ensures(true)]
fn collect_cmavo_variants(tokens: &TokenStream, variants: &mut BTreeSet<String>) {
    let values = tokens.clone().into_iter().collect::<Vec<_>>();
    for window in values.windows(4) {
        if let [
            TokenTree::Ident(ident),
            TokenTree::Punct(equal),
            TokenTree::Punct(angle),
            TokenTree::Group(group),
        ] = window
            && equal.as_char() == '='
            && angle.as_char() == '>'
            && group.delimiter() == Delimiter::Brace
            && cmavo_entry_body(group)
        {
            variants.insert(ident.to_string());
        }
    }
    for token in values {
        if let TokenTree::Group(group) = token {
            collect_cmavo_variants(&group.stream(), variants);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn cmavo_entry_body(group: &Group) -> bool {
    let identifiers = group
        .stream()
        .into_iter()
        .filter_map(|token| match token {
            TokenTree::Ident(ident) => Some(ident.to_string()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    identifiers.contains("text") && identifiers.contains("selmaho")
}

#[requires(true)]
#[ensures(true)]
fn print_inventory(inventory: &[ApiItem]) {
    for item in inventory {
        println!(
            "{}\t{}\t{}\t{}",
            item.rust_path,
            item.kind,
            item.rust_signature,
            item.suggested_python.as_deref().unwrap_or("")
        );
    }
}

#[requires(!inventory.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn write_template(inventory: &[ApiItem], matrix_path: &Path) -> Result<(), String> {
    if matrix_path.exists() {
        return Err(format!(
            "refusing to overwrite existing matrix {}",
            matrix_path.display()
        ));
    }
    let mut output = String::from(MATRIX_HEADER);
    output.push('\n');
    for item in inventory {
        let python = item.suggested_python.as_deref().unwrap_or("");
        writeln!(
            output,
            "{}\t{}\t{}\tUNCLASSIFIED\t{}\t\t",
            item.rust_path, item.kind, item.rust_signature, python
        )
        .expect("writing to String cannot fail");
    }
    fs::write(matrix_path, output)
        .map_err(|error| format!("failed to write {}: {error}", matrix_path.display()))
}

#[requires(!inventory.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn check_matrix(inventory: &[ApiItem], matrix_path: &Path) -> Result<(), String> {
    let entries = read_matrix(matrix_path)?;
    let inventory_by_key = inventory
        .iter()
        .map(|item| ((item.rust_path.as_str(), item.kind.as_str()), item))
        .collect::<BTreeMap<_, _>>();
    let entries_by_key = entries
        .iter()
        .map(|entry| ((entry.rust_path.as_str(), entry.kind.as_str()), entry))
        .collect::<BTreeMap<_, _>>();
    let mut errors = Vec::new();
    for ((path, kind), item) in &inventory_by_key {
        let Some(entry) = entries_by_key.get(&(*path, *kind)) else {
            errors.push(format!("unclassified Rust API item: {path} [{kind}]"));
            continue;
        };
        if entry.rust_signature != item.rust_signature {
            errors.push(format!(
                "stale Rust API signature for {path} [{kind}]: inventory={}, matrix={}",
                item.rust_signature, entry.rust_signature
            ));
        }
    }
    for (path, kind) in entries_by_key.keys() {
        if !inventory_by_key.contains_key(&(*path, *kind)) {
            errors.push(format!(
                "matrix contains removed Rust API item: {path} [{kind}]"
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|entries| !entries.is_empty()) || ret.is_err())]
fn read_matrix(matrix_path: &Path) -> Result<Vec<MatrixEntry>, String> {
    let source = fs::read_to_string(matrix_path)
        .map_err(|error| format!("failed to read {}: {error}", matrix_path.display()))?;
    let mut lines = source.lines();
    if lines.next() != Some(MATRIX_HEADER) {
        return Err(format!(
            "{} does not begin with the required matrix header",
            matrix_path.display()
        ));
    }
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 {
            return Err(format!(
                "{}:{} has {} TSV fields; expected 7",
                matrix_path.display(),
                index + 2,
                fields.len()
            ));
        }
        let entry = try_new!(MatrixEntry {
            rust_path: fields[0].to_owned(),
            kind: fields[1].to_owned(),
            rust_signature: fields[2].to_owned(),
            disposition: fields[3].to_owned(),
            python_path: fields[4].to_owned(),
            exclusion_kind: fields[5].to_owned(),
            rationale: fields[6].to_owned(),
        })
        .map_err(|error| {
            format!(
                "{}:{} contains an invalid matrix row: {error}",
                matrix_path.display(),
                index + 2
            )
        })?;
        validate_matrix_entry(&entry, matrix_path, index + 2)?;
        if !seen.insert((entry.rust_path.clone(), entry.kind.clone())) {
            return Err(format!(
                "{}:{} duplicates {} [{}]",
                matrix_path.display(),
                index + 2,
                entry.rust_path,
                entry.kind
            ));
        }
        entries.push(entry);
    }
    Ok(entries)
}

#[requires(!entry.rust_path.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_matrix_entry(
    entry: &MatrixEntry,
    matrix_path: &Path,
    line_number: usize,
) -> Result<(), String> {
    let location = format!("{}:{line_number}", matrix_path.display());
    if !DISPOSITIONS.contains(&entry.disposition.as_str()) {
        return Err(format!(
            "{location} has invalid disposition `{}`",
            entry.disposition
        ));
    }
    if entry.disposition == "rust-only" {
        if !entry.python_path.is_empty() {
            return Err(format!(
                "{location} rust-only entry must not name a Python path"
            ));
        }
        if !EXCLUSION_KINDS.contains(&entry.exclusion_kind.as_str()) {
            return Err(format!(
                "{location} has invalid rust-only exclusion kind `{}`",
                entry.exclusion_kind
            ));
        }
        if entry.rationale.is_empty() {
            return Err(format!(
                "{location} rust-only entry requires a concrete rationale"
            ));
        }
    } else {
        if entry.python_path.is_empty() {
            return Err(format!(
                "{location} {} entry must name its Python operation",
                entry.disposition
            ));
        }
        if !entry.exclusion_kind.is_empty() {
            return Err(format!(
                "{location} non-exclusion entry must not name an exclusion kind"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_schema_inventory_is_nonempty_and_unique() {
        assert!(!GENERATED_SYNTAX_API.is_empty());
        let paths = GENERATED_SYNTAX_API
            .iter()
            .map(|(path, _, _, _)| *path)
            .collect::<BTreeSet<_>>();
        assert_eq!(paths.len(), GENERATED_SYNTAX_API.len());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn arrow_variant_parser_uses_token_structure() {
        let tokens: TokenStream = syn::parse_str("Alpha => \"a\", Beta => (\"BETA\", \"b\")")
            .expect("valid token stream");
        assert_eq!(parse_arrow_variants(&tokens), ["Alpha", "Beta"]);
    }
}

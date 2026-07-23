use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use bityzba::{contract_trait, ensures, invariant, new, requires};
use quote::ToTokens;
use syn::parse::Parser as _;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprLit, Fields, ForeignItem, ImplItem, Item, Lit, Meta, TraitItem,
    Visibility,
};

use super::model::{
    ContractRecord, EdgeKind, EdgeRecord, FieldRecord, FieldStyle, FileClass, FunctionKind,
    FunctionRecord, LoweringSiteRecord, ModuleRecord, RendererParserConsumerKind,
    RendererParserConsumerRecord, SerializationKind, SerializationRecord, SourceIdentity,
    TestFixtureKind, TestFixtureRecord, TypeKind, TypeRecord, VariantRecord,
};
use super::source::{SourceMap, record_id};

const CONTRACT_ATTRIBUTES: &[&str] = &[
    "contract_trait",
    "ensures",
    "expensive_ensures",
    "expensive_invariant",
    "expensive_requires",
    "invariant",
    "requires",
    "test_ensures",
    "test_invariant",
    "test_requires",
];

const SERIALIZATION_FORMAT_CONSTANTS: &[&str] = &["MCP_PROTOCOL_VERSION", "SEMANTIC_JSON_VERSION"];

const SEMANTIC_RENDER_CALLS: &[&str] = &[
    "json_string_with_options",
    "render_tree",
    "render_tree_proj",
    "render_tersmu",
    "to_compact_json",
    "to_pretty_json",
];

const SEMANTIC_PARSE_CALLS: &[&str] = &[
    "parse_syntax_tree_generated_model_with_source_and_options",
    "parse_syntax_tree_with_recovery_with_source_and_options_attempt",
];

const REGISTERED_CONSUMER_DECLARATIONS: &[(&str, &str, RendererParserConsumerKind)] = &[
    (
        "apps/jbotci/src/commands/tersmu.rs",
        "run_tersmu",
        RendererParserConsumerKind::CliSurface,
    ),
    (
        "apps/jbotci/src/commands/tersmu.rs",
        "render_tersmu",
        RendererParserConsumerKind::CliSurface,
    ),
    (
        "apps/jbotci/src/tool.rs",
        "run_tool_tersmu",
        RendererParserConsumerKind::CliSurface,
    ),
    (
        "apps/jbotci-server/src/lib.rs",
        "tersmu",
        RendererParserConsumerKind::McpSurface,
    ),
    (
        "apps/jbotci-server/src/mcp.rs",
        "call_tool",
        RendererParserConsumerKind::McpSurface,
    ),
    (
        "xtask-full/src/semantics_coverage.rs",
        "analyze_fixture",
        RendererParserConsumerKind::FixtureHarness,
    ),
];

#[invariant(true)]
#[derive(Debug, Default)]
pub(crate) struct RustInventory {
    pub(crate) modules: Vec<ModuleRecord>,
    pub(crate) types: Vec<TypeRecord>,
    pub(crate) variants: Vec<VariantRecord>,
    pub(crate) fields: Vec<FieldRecord>,
    pub(crate) functions: Vec<FunctionRecord>,
    pub(crate) contracts: Vec<ContractRecord>,
    pub(crate) serialization: Vec<SerializationRecord>,
    pub(crate) edges: Vec<EdgeRecord>,
    pub(crate) lowering_sites: Vec<LoweringSiteRecord>,
    pub(crate) consumers: Vec<RendererParserConsumerRecord>,
    pub(crate) tests: Vec<TestFixtureRecord>,
    pub(crate) declaration_owners: BTreeSet<String>,
}

#[invariant(!id.is_empty() && !name.is_empty())]
#[derive(Debug, Clone)]
struct ModuleContext {
    id: String,
    name: String,
}

#[requires(!path.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn extract_rust_file(
    path: &str,
    source: &str,
    class: FileClass,
    inventory: &mut RustInventory,
) -> Result<()> {
    let syntax = syn::parse_file(source).with_context(|| format!("parsing pinned Rust `{path}`"))?;
    let source_map = SourceMap::new(path, source);
    let root_source = source_map.whole_file();
    let root_id = record_id("module", &root_source);
    let root_name = module_name_from_path(path)?;
    inventory.modules.push(new!(ModuleRecord {
        id: root_id.clone(),
        source: root_source,
        name: root_name.clone(),
        parent: None,
        declared_path: Some(path.to_owned()),
        inline: false,
    }));
    inventory.declaration_owners.insert(root_id.clone());
    let mut extractor = RustExtractor {
        source_map: &source_map,
        class,
        inventory,
        modules: vec![new!(ModuleContext {
            id: root_id.clone(),
            name: root_name,
        })],
    };
    extractor.attributes(&root_id, "module", &syntax.attrs)?;
    extractor.items(&syntax.items)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn validate_extraction_completeness(inventory: &RustInventory) -> Result<()> {
    let mut registered_counts = REGISTERED_CONSUMER_DECLARATIONS
        .iter()
        .map(|(path, name, kind)| ((*path, *name, *kind), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for record in inventory
        .consumers
        .iter()
        .filter(|record| record.id.starts_with("registered-consumer:"))
    {
        let key = (
            record.source.path.as_str(),
            record.symbol.as_str(),
            record.kind,
        );
        if let Some(count) = registered_counts.get_mut(&key) {
            *count += 1;
        }
    }
    let invalid_registered = registered_counts
        .iter()
        .filter(|(_, count)| **count != 1)
        .map(|(key, count)| format!("{key:?} matched {count} records"))
        .collect::<Vec<_>>();
    if !invalid_registered.is_empty() {
        bail!(
            "registered Rust executable-consumer declarations are not one-to-one: {}",
            invalid_registered.join("; ")
        );
    }

    let format_constants = inventory
        .serialization
        .iter()
        .filter(|record| record.kind == SerializationKind::FormatConstant)
        .filter_map(|record| record.key.as_deref())
        .collect::<Vec<_>>();
    for name in SERIALIZATION_FORMAT_CONSTANTS {
        if format_constants.iter().filter(|actual| *actual == name).count() != 1 {
            bail!("registered serialization constant `{name}` is not inventoried exactly once");
        }
    }
    Ok(())
}

#[invariant(!modules.is_empty())]
struct RustExtractor<'map, 'source, 'inventory> {
    source_map: &'map SourceMap<'source>,
    class: FileClass,
    inventory: &'inventory mut RustInventory,
    modules: Vec<ModuleContext>,
}

impl RustExtractor<'_, '_, '_> {
    #[requires(!self.modules.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn items(&mut self, items: &[Item]) -> Result<()> {
        for item in items {
            self.item(item)?;
        }
        Ok(())
    }

    #[requires(!self.modules.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Mod(item) => self.item_mod(item),
            Item::Struct(item) => {
                let source = self.source_map.span(item.span())?;
                let id = record_id("type", &source);
                self.push_type(
                    id.clone(),
                    source.clone(),
                    item.ident.to_string(),
                    TypeKind::Struct,
                    &item.vis,
                    item.generics.to_token_stream().to_string(),
                    &item.attrs,
                )?;
                self.fields(&id, &item.fields, &source)
            }
            Item::Enum(item) => {
                let source = self.source_map.span(item.span())?;
                let id = record_id("type", &source);
                self.push_type(
                    id.clone(),
                    source,
                    item.ident.to_string(),
                    TypeKind::Enum,
                    &item.vis,
                    item.generics.to_token_stream().to_string(),
                    &item.attrs,
                )?;
                for variant in &item.variants {
                    let source = self.source_map.span(variant.span())?;
                    let variant_id = record_id("variant", &source);
                    self.inventory.variants.push(new!(VariantRecord {
                        id: variant_id.clone(),
                        source: source.clone(),
                        type_id: id.clone(),
                        name: variant.ident.to_string(),
                        discriminant: variant
                            .discriminant
                            .as_ref()
                            .map(|(_, expression)| expression.to_token_stream().to_string()),
                    }));
                    self.inventory
                        .declaration_owners
                        .insert(variant_id.clone());
                    self.attributes(&variant_id, "variant", &variant.attrs)?;
                    self.fields(&variant_id, &variant.fields, &source)?;
                }
                Ok(())
            }
            Item::Union(item) => {
                let source = self.source_map.span(item.span())?;
                let id = record_id("type", &source);
                self.push_type(
                    id.clone(),
                    source,
                    item.ident.to_string(),
                    TypeKind::Union,
                    &item.vis,
                    item.generics.to_token_stream().to_string(),
                    &item.attrs,
                )?;
                self.named_fields(&id, &item.fields.named)
            }
            Item::Type(item) => {
                let source = self.source_map.span(item.span())?;
                let id = record_id("type", &source);
                self.push_type(
                    id,
                    source,
                    item.ident.to_string(),
                    TypeKind::Alias,
                    &item.vis,
                    item.generics.to_token_stream().to_string(),
                    &item.attrs,
                )
            }
            Item::Trait(item) => self.item_trait(item),
            Item::TraitAlias(item) => {
                let source = self.source_map.span(item.span())?;
                let id = record_id("type", &source);
                self.push_type(
                    id,
                    source,
                    item.ident.to_string(),
                    TypeKind::TraitAlias,
                    &item.vis,
                    item.generics.to_token_stream().to_string(),
                    &item.attrs,
                )
            }
            Item::Fn(item) => self.function(
                &item.sig,
                &item.vis,
                &item.attrs,
                FunctionKind::Free,
                None,
                Some(&item.block),
            ),
            Item::Impl(item) => self.item_impl(item),
            Item::Const(item) => {
                let source = self.source_map.span(item.span())?;
                let owner = record_id("const", &source);
                self.inventory.declaration_owners.insert(owner.clone());
                self.attributes(&owner, "const", &item.attrs)?;
                self.format_constant(&owner, &item.ident.to_string(), source, &item.expr)
            }
            Item::Static(item) => {
                let source = self.source_map.span(item.span())?;
                let owner = record_id("static", &source);
                self.inventory.declaration_owners.insert(owner.clone());
                self.attributes(&owner, "static", &item.attrs)?;
                self.format_constant(&owner, &item.ident.to_string(), source, &item.expr)
            }
            Item::ForeignMod(item) => {
                let source = self.source_map.span(item.span())?;
                let owner = record_id("foreign-module", &source);
                self.inventory.declaration_owners.insert(owner.clone());
                self.attributes(&owner, "foreign-module", &item.attrs)?;
                for foreign_item in &item.items {
                    match foreign_item {
                        ForeignItem::Fn(function) => self.function(
                            &function.sig,
                            &function.vis,
                            &function.attrs,
                            FunctionKind::Foreign,
                            None,
                            None,
                        )?,
                        ForeignItem::Type(item_type) => {
                            let source = self.source_map.span(item_type.span())?;
                            let id = record_id("type", &source);
                            self.push_type(
                                id,
                                source,
                                item_type.ident.to_string(),
                                TypeKind::ForeignType,
                                &item_type.vis,
                                item_type.generics.to_token_stream().to_string(),
                                &item_type.attrs,
                            )?;
                        }
                        ForeignItem::Verbatim(_) => bail!(
                            "unsupported verbatim foreign item in `{}`",
                            self.source_map.path()
                        ),
                        ForeignItem::Static(_) | ForeignItem::Macro(_) => {
                            self.generic_item_attributes(foreign_item, "foreign-item")?
                        }
                        _ => bail!(
                            "unsupported foreign item shape in `{}`",
                            self.source_map.path()
                        ),
                    }
                }
                Ok(())
            }
            Item::Verbatim(_) => bail!(
                "unsupported verbatim Rust item in `{}`",
                self.source_map.path()
            ),
            Item::ExternCrate(_) => self.generic_item_attributes(item, "extern-crate"),
            Item::Macro(_) => self.generic_item_attributes(item, "macro"),
            Item::Use(_) => self.generic_item_attributes(item, "use"),
            _ => bail!("unsupported Rust item shape in `{}`", self.source_map.path()),
        }
    }

    #[requires(!self.modules.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn item_mod(&mut self, item: &syn::ItemMod) -> Result<()> {
        let source = self.source_map.span(item.span())?;
        let id = record_id("module", &source);
        let parent = self.current_module().id.clone();
        let name = format!("{}::{}", self.current_module().name, item.ident);
        self.inventory.modules.push(new!(ModuleRecord {
            id: id.clone(),
            source,
            name: name.clone(),
            parent: Some(parent),
            declared_path: path_attribute(&item.attrs),
            inline: item.content.is_some(),
        }));
        self.inventory.declaration_owners.insert(id.clone());
        self.attributes(&id, "module", &item.attrs)?;
        if let Some((_, items)) = &item.content {
            self.modules.push(new!(ModuleContext { id, name }));
            let result = self.items(items);
            self.modules.pop();
            result?;
        }
        Ok(())
    }

    #[requires(!self.modules.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn item_trait(&mut self, item: &syn::ItemTrait) -> Result<()> {
        let source = self.source_map.span(item.span())?;
        let type_id = record_id("type", &source);
        self.push_type(
            type_id.clone(),
            source,
            item.ident.to_string(),
            TypeKind::Trait,
            &item.vis,
            item.generics.to_token_stream().to_string(),
            &item.attrs,
        )?;
        for trait_item in &item.items {
            match trait_item {
                TraitItem::Fn(function) => self.function(
                    &function.sig,
                    &Visibility::Inherited,
                    &function.attrs,
                    FunctionKind::TraitDeclaration,
                    Some(item.ident.to_string()),
                    function.default.as_ref(),
                )?,
                TraitItem::Type(item_type) => {
                    let source = self.source_map.span(item_type.span())?;
                    let id = record_id("type", &source);
                    self.push_type(
                        id,
                        source,
                        format!("{}::{}", item.ident, item_type.ident),
                        TypeKind::AssociatedType,
                        &Visibility::Inherited,
                        item_type.generics.to_token_stream().to_string(),
                        &item_type.attrs,
                    )?;
                }
                TraitItem::Verbatim(_) => bail!(
                    "unsupported verbatim trait item in `{}`",
                    self.source_map.path()
                ),
                TraitItem::Const(_) | TraitItem::Macro(_) => {
                    self.generic_item_attributes(trait_item, "trait-item")?
                }
                _ => bail!(
                    "unsupported trait item shape in `{}`",
                    self.source_map.path()
                ),
            }
        }
        Ok(())
    }

    #[requires(!self.modules.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn item_impl(&mut self, item: &syn::ItemImpl) -> Result<()> {
        let source = self.source_map.span(item.span())?;
        let impl_id = record_id("impl", &source);
        self.inventory.declaration_owners.insert(impl_id.clone());
        let owner_type = item.self_ty.to_token_stream().to_string();
        let owner_name = terminal_type_name(&item.self_ty);
        self.attributes(&impl_id, "impl", &item.attrs)?;
        let trait_name = item
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last())
            .map(|segment| segment.ident.to_string());
        if matches!(trait_name.as_deref(), Some("Serialize" | "Deserialize")) {
            let kind = if trait_name.as_deref() == Some("Serialize") {
                SerializationKind::CustomSerializeImplementation
            } else {
                SerializationKind::CustomDeserializeImplementation
            };
            self.inventory.serialization.push(new!(SerializationRecord {
                id: record_id("serialization", &source),
                source,
                owner_id: impl_id,
                kind,
                detail: owner_type.clone(),
                key: None,
            }));
        }
        for impl_item in &item.items {
            match impl_item {
                ImplItem::Fn(function) => self.function(
                    &function.sig,
                    &function.vis,
                    &function.attrs,
                    if item.trait_.is_some() {
                        FunctionKind::TraitMethod
                    } else {
                        FunctionKind::InherentMethod
                    },
                    owner_name.clone().or_else(|| Some(owner_type.clone())),
                    Some(&function.block),
                )?,
                ImplItem::Const(constant) => {
                    let source = self.source_map.span(constant.span())?;
                    let owner = record_id("associated-const", &source);
                    self.inventory.declaration_owners.insert(owner.clone());
                    self.attributes(&owner, "associated-const", &constant.attrs)?;
                    self.format_constant(
                        &owner,
                        &constant.ident.to_string(),
                        source,
                        &constant.expr,
                    )?;
                }
                ImplItem::Type(item_type) => {
                    let source = self.source_map.span(item_type.span())?;
                    let id = record_id("type", &source);
                    self.push_type(
                        id,
                        source,
                        format!("{}::{}", owner_type, item_type.ident),
                        TypeKind::AssociatedType,
                        &item_type.vis,
                        item_type.generics.to_token_stream().to_string(),
                        &item_type.attrs,
                    )?;
                }
                ImplItem::Verbatim(_) => bail!(
                    "unsupported verbatim impl item in `{}`",
                    self.source_map.path()
                ),
                ImplItem::Macro(_) => self.generic_item_attributes(impl_item, "impl-item")?,
                _ => bail!(
                    "unsupported impl item shape in `{}`",
                    self.source_map.path()
                ),
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[requires(!id.is_empty() && !name.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn push_type(
        &mut self,
        id: String,
        source: SourceIdentity,
        name: String,
        kind: TypeKind,
        visibility: &Visibility,
        generic_parameters: String,
        attributes: &[Attribute],
    ) -> Result<()> {
        self.inventory.types.push(new!(TypeRecord {
            id: id.clone(),
            source,
            module_id: self.current_module().id.clone(),
            name,
            kind,
            visibility: visibility.to_token_stream().to_string(),
            generic_parameters,
        }));
        self.inventory.declaration_owners.insert(id.clone());
        self.attributes(&id, "type", attributes)
    }

    #[requires(!owner_id.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn fields(
        &mut self,
        owner_id: &str,
        fields: &Fields,
        owner_source: &SourceIdentity,
    ) -> Result<()> {
        match fields {
            Fields::Named(fields) => self.named_fields(owner_id, &fields.named),
            Fields::Unnamed(fields) => {
                for (index, field) in fields.unnamed.iter().enumerate() {
                    self.field(owner_id, index.to_string(), FieldStyle::Tuple, field)?;
                }
                Ok(())
            }
            Fields::Unit => {
                let field_id = format!("field:{}#unit", owner_source.stable_id());
                self.inventory.fields.push(new!(FieldRecord {
                    id: field_id.clone(),
                    source: owner_source.clone(),
                    owner_id: owner_id.to_owned(),
                    name: "$unit".to_owned(),
                    style: FieldStyle::Unit,
                    visibility: String::new(),
                    rust_type: "()".to_owned(),
                }));
                self.inventory
                    .declaration_owners
                    .insert(field_id);
                Ok(())
            }
        }
    }

    #[requires(!owner_id.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn named_fields(
        &mut self,
        owner_id: &str,
        fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    ) -> Result<()> {
        for field in fields {
            let name = field
                .ident
                .as_ref()
                .context("named Rust field lacks an identifier")?
                .to_string();
            self.field(owner_id, name, FieldStyle::Named, field)?;
        }
        Ok(())
    }

    #[requires(!owner_id.is_empty() && !name.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn field(
        &mut self,
        owner_id: &str,
        name: String,
        style: FieldStyle,
        field: &syn::Field,
    ) -> Result<()> {
        let source = self.source_map.span(field.span())?;
        let field_id = record_id("field", &source);
        self.inventory.fields.push(new!(FieldRecord {
            id: field_id.clone(),
            source: source.clone(),
            owner_id: owner_id.to_owned(),
            name,
            style,
            visibility: field.vis.to_token_stream().to_string(),
            rust_type: field.ty.to_token_stream().to_string(),
        }));
        self.inventory.declaration_owners.insert(field_id.clone());
        self.attributes(&field_id, "field", &field.attrs)?;
        let mut paths = TypePathCollector::default();
        paths.visit_type(&field.ty);
        for target in paths.paths {
            self.inventory.edges.push(new!(EdgeRecord {
                id: format!("edge:{}#{target}", source.stable_id()),
                source: source.clone(),
                owner_id: field_id.clone(),
                kind: EdgeKind::FieldType,
                target,
            }));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[requires(!self.modules.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn function(
        &mut self,
        signature: &syn::Signature,
        visibility: &Visibility,
        attributes: &[Attribute],
        kind: FunctionKind,
        owner_type: Option<String>,
        body: Option<&syn::Block>,
    ) -> Result<()> {
        let source = self.source_map.span(signature.span())?;
        let id = record_id("function", &source);
        let is_test = attributes.iter().any(|attribute| attribute.path().is_ident("test"));
        self.inventory.functions.push(new!(FunctionRecord {
            id: id.clone(),
            source: source.clone(),
            module_id: self.current_module().id.clone(),
            owner_type,
            name: signature.ident.to_string(),
            kind,
            visibility: visibility.to_token_stream().to_string(),
            signature: self.source_map.slice(signature.span())?.to_owned(),
            is_async: signature.asyncness.is_some(),
            is_unsafe: signature.unsafety.is_some(),
            is_test,
        }));
        self.inventory.declaration_owners.insert(id.clone());
        if let Some(kind) = registered_consumer_declaration(
            self.source_map.path(),
            &signature.ident.to_string(),
        ) {
            self.inventory.consumers.push(new!(RendererParserConsumerRecord {
                id: record_id("registered-consumer", &source),
                source: source.clone(),
                owner_id: id.clone(),
                kind,
                symbol: signature.ident.to_string(),
            }));
        }
        self.attributes(&id, "function", attributes)?;
        if is_test {
            self.inventory.tests.push(new!(TestFixtureRecord {
                id: record_id("test", &source),
                source: source.clone(),
                name: signature.ident.to_string(),
                kind: TestFixtureKind::RustTest,
                owner_id: Some(id.clone()),
                semantic_reference_expectation: false,
                tersmu_output_expectation: false,
            }));
        }
        if matches!(self.class, FileClass::RustLowering) {
            self.inventory.lowering_sites.push(new!(LoweringSiteRecord {
                id: record_id("lowering-function", &source),
                source: source.clone(),
                function_id: id.clone(),
                operation: "lowering-function".to_owned(),
                constructed_type: None,
            }));
        }
        if matches!(self.class, FileClass::RustRenderer) {
            self.inventory.consumers.push(new!(RendererParserConsumerRecord {
                id: record_id("renderer-consumer", &source),
                source: source.clone(),
                owner_id: id.clone(),
                kind: RendererParserConsumerKind::Renderer,
                symbol: signature.ident.to_string(),
            }));
        }
        if let Some(body) = body {
            let mut visitor = FunctionBodyVisitor {
                source_map: self.source_map,
                class: self.class,
                function_id: &id,
                lowering_sites: &mut self.inventory.lowering_sites,
                consumers: &mut self.inventory.consumers,
                serialization: &mut self.inventory.serialization,
                edges: &mut self.inventory.edges,
                errors: Vec::new(),
            };
            visitor.visit_block(body);
            visitor.finish()?;
        }
        Ok(())
    }

    #[requires(!owner_id.is_empty() && !owner_kind.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn attributes(
        &mut self,
        owner_id: &str,
        owner_kind: &str,
        attributes: &[Attribute],
    ) -> Result<()> {
        for attribute in attributes {
            let Some(name) = attribute.path().segments.last().map(|segment| segment.ident.to_string())
            else {
                bail!("Rust attribute path is empty");
            };
            let source = self.source_map.span(attribute.span())?;
            let exact = self.source_map.slice(attribute.span())?.to_owned();
            if CONTRACT_ATTRIBUTES.contains(&name.as_str()) {
                self.inventory.contracts.push(new!(ContractRecord {
                    id: record_id("contract", &source),
                    source: source.clone(),
                    owner_id: owner_id.to_owned(),
                    owner_kind: owner_kind.to_owned(),
                    contract_kind: name.clone(),
                    attribute: exact.clone(),
                }));
            }
            if name == "serde" {
                self.inventory.serialization.push(new!(SerializationRecord {
                    id: record_id("serialization", &source),
                    source: source.clone(),
                    owner_id: owner_id.to_owned(),
                    kind: SerializationKind::SerdeAttribute,
                    detail: exact,
                    key: None,
                }));
                self.serde_details(owner_id, attribute)?;
            }
        }
        Ok(())
    }

    #[requires(!owner_id.is_empty() && attribute.path().is_ident("serde"))]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn serde_details(&mut self, owner_id: &str, attribute: &Attribute) -> Result<()> {
        let Meta::List(list) = &attribute.meta else {
            return Ok(());
        };
        let nested = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())?;
        for meta in nested {
            let Some(name) = meta
                .path()
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
            else {
                bail!("serde metadata path is empty");
            };
            let kind = if matches!(name.as_str(), "rename" | "rename_all" | "tag" | "content") {
                Some(SerializationKind::SerdeNaming)
            } else if matches!(
                name.as_str(),
                "serialize_with" | "deserialize_with" | "with"
            ) {
                Some(SerializationKind::SerdeCustomCodec)
            } else if matches!(
                name.as_str(),
                "skip"
                    | "skip_serializing"
                    | "skip_deserializing"
                    | "skip_serializing_if"
                    | "default"
            ) {
                Some(SerializationKind::SerdeOmission)
            } else if name == "flatten" {
                Some(SerializationKind::SerdeFlattening)
            } else {
                None
            };
            let Some(kind) = kind else {
                continue;
            };
            let source = self.source_map.span(meta.span())?;
            self.inventory.serialization.push(new!(SerializationRecord {
                id: record_id("serialization-serde-detail", &source),
                source,
                owner_id: owner_id.to_owned(),
                kind,
                detail: self.source_map.slice(meta.span())?.to_owned(),
                key: serde_string_value(&meta),
            }));
        }
        Ok(())
    }

    #[requires(!owner.is_empty() && !name.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn format_constant(
        &mut self,
        owner: &str,
        name: &str,
        source: SourceIdentity,
        expression: &Expr,
    ) -> Result<()> {
        if SERIALIZATION_FORMAT_CONSTANTS.contains(&name) {
            self.inventory.serialization.push(new!(SerializationRecord {
                id: record_id("serialization-constant", &source),
                source,
                owner_id: owner.to_owned(),
                kind: SerializationKind::FormatConstant,
                detail: expression.to_token_stream().to_string(),
                key: Some(name.to_owned()),
            }));
        }
        Ok(())
    }

    #[requires(!owner_kind.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn generic_item_attributes<T>(&mut self, item: &T, owner_kind: &str) -> Result<()>
    where
        T: Spanned + AttributeOwner,
    {
        let source = self.source_map.span(item.span())?;
        let owner = record_id(owner_kind, &source);
        self.inventory.declaration_owners.insert(owner.clone());
        self.attributes(&owner, owner_kind, item.attributes())
    }

    #[requires(!self.modules.is_empty())]
    #[ensures(!ret.id.is_empty())]
    fn current_module(&self) -> &ModuleContext {
        self.modules
            .last()
            .expect("module stack is nonempty by construction")
    }
}

#[contract_trait]
trait AttributeOwner {
    #[requires(true)]
    #[ensures(true)]
    fn attributes(&self) -> &[Attribute];
}

#[contract_trait]
impl AttributeOwner for Item {
    fn attributes(&self) -> &[Attribute] {
        match self {
            Item::Const(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::ExternCrate(item) => &item.attrs,
            Item::Fn(item) => &item.attrs,
            Item::ForeignMod(item) => &item.attrs,
            Item::Impl(item) => &item.attrs,
            Item::Macro(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Trait(item) => &item.attrs,
            Item::TraitAlias(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            Item::Union(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            Item::Verbatim(_) => &[],
            _ => &[],
        }
    }
}

#[contract_trait]
impl AttributeOwner for ForeignItem {
    fn attributes(&self) -> &[Attribute] {
        match self {
            ForeignItem::Fn(item) => &item.attrs,
            ForeignItem::Static(item) => &item.attrs,
            ForeignItem::Type(item) => &item.attrs,
            ForeignItem::Macro(item) => &item.attrs,
            ForeignItem::Verbatim(_) => &[],
            _ => &[],
        }
    }
}

#[contract_trait]
impl AttributeOwner for ImplItem {
    fn attributes(&self) -> &[Attribute] {
        match self {
            ImplItem::Const(item) => &item.attrs,
            ImplItem::Fn(item) => &item.attrs,
            ImplItem::Type(item) => &item.attrs,
            ImplItem::Macro(item) => &item.attrs,
            ImplItem::Verbatim(_) => &[],
            _ => &[],
        }
    }
}

#[contract_trait]
impl AttributeOwner for TraitItem {
    fn attributes(&self) -> &[Attribute] {
        match self {
            TraitItem::Const(item) => &item.attrs,
            TraitItem::Fn(item) => &item.attrs,
            TraitItem::Type(item) => &item.attrs,
            TraitItem::Macro(item) => &item.attrs,
            TraitItem::Verbatim(_) => &[],
            _ => &[],
        }
    }
}

#[invariant(true)]
#[derive(Default)]
struct TypePathCollector {
    paths: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for TypePathCollector {
    #[requires(true)]
    #[ensures(true)]
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if let Some(segment) = node.path.segments.last() {
            self.paths.insert(segment.ident.to_string());
        }
        visit::visit_type_path(self, node);
    }
}

#[invariant(!function_id.is_empty())]
struct FunctionBodyVisitor<'map, 'source, 'function, 'records> {
    source_map: &'map SourceMap<'source>,
    class: FileClass,
    function_id: &'function str,
    lowering_sites: &'records mut Vec<LoweringSiteRecord>,
    consumers: &'records mut Vec<RendererParserConsumerRecord>,
    serialization: &'records mut Vec<SerializationRecord>,
    edges: &'records mut Vec<EdgeRecord>,
    errors: Vec<String>,
}

impl FunctionBodyVisitor<'_, '_, '_, '_> {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn finish(self) -> Result<()> {
        if !self.errors.is_empty() {
            bail!(
                "failed to map syntax positions while walking `{}`: {}",
                self.source_map.path(),
                self.errors.join("; ")
            );
        }
        Ok(())
    }

    #[requires(!name.is_empty())]
    #[ensures(true)]
    fn call(&mut self, name: &str, span: proc_macro2::Span, first_argument: Option<&Expr>) {
        let source = match self.source_map.span(span) {
            Ok(source) => source,
            Err(error) => {
                self.errors.push(error.to_string());
                return;
            }
        };
        if name == "references_into" {
            self.edges.push(new!(EdgeRecord {
                id: record_id("reference-edge", &source),
                source: source.clone(),
                owner_id: self.function_id.to_owned(),
                kind: EdgeKind::ReferenceCollection,
                target: name.to_owned(),
            }));
        } else if name == "visit_in_order" {
            self.edges.push(new!(EdgeRecord {
                id: record_id("tree-visit-edge", &source),
                source: source.clone(),
                owner_id: self.function_id.to_owned(),
                kind: EdgeKind::TreeVisit,
                target: name.to_owned(),
            }));
        } else if name == "walk_node" {
            self.edges.push(new!(EdgeRecord {
                id: record_id("walker-edge", &source),
                source: source.clone(),
                owner_id: self.function_id.to_owned(),
                kind: EdgeKind::WalkerDescent,
                target: name.to_owned(),
            }));
        }
        if matches!(name, "serialize_entry" | "serialize_field") {
            self.serialization.push(new!(SerializationRecord {
                id: record_id("serialized-key", &source),
                source: source.clone(),
                owner_id: self.function_id.to_owned(),
                kind: SerializationKind::SerializedKey,
                detail: first_argument
                    .map(|expression| expression.to_token_stream())
                    .map(|tokens| tokens.to_string())
                    .unwrap_or_else(|| name.to_owned()),
                key: first_argument.and_then(literal_string_expression),
            }));
        }
        let consumer_kind = if SEMANTIC_RENDER_CALLS.contains(&name) {
            Some(RendererParserConsumerKind::Renderer)
        } else if SEMANTIC_PARSE_CALLS.contains(&name) {
            Some(RendererParserConsumerKind::Parser)
        } else if name == "run_tool_tersmu" {
            Some(RendererParserConsumerKind::CliSurface)
        } else {
            None
        };
        if let Some(kind) = consumer_kind {
            let kind = match (self.class, self.source_map.path()) {
                (FileClass::RustValidationTool, _) => {
                    RendererParserConsumerKind::FixtureHarness
                }
                (
                    FileClass::RustConsumer,
                    "apps/jbotci-server/src/lib.rs" | "apps/jbotci-server/src/mcp.rs",
                ) => RendererParserConsumerKind::McpSurface,
                (FileClass::RustConsumer, _) => RendererParserConsumerKind::CliSurface,
                _ => kind,
            };
            self.consumers.push(new!(RendererParserConsumerRecord {
                id: record_id("renderer-parser-consumer", &source),
                source,
                owner_id: self.function_id.to_owned(),
                kind,
                symbol: name.to_owned(),
            }));
        }
    }
}

impl<'ast> Visit<'ast> for FunctionBodyVisitor<'_, '_, '_, '_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            if let Some(segment) = path.path.segments.last() {
                self.call(&segment.ident.to_string(), node.span(), node.args.first());
            }
        }
        visit::visit_expr_call(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.call(&node.method.to_string(), node.span(), node.args.first());
        visit::visit_expr_method_call(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if matches!(self.class, FileClass::RustLowering) && matches!(name.as_str(), "new" | "try_new")
        {
            match (
                self.source_map.span(node.span()),
                macro_constructed_type(node),
            ) {
                (Ok(source), Ok(constructed_type)) => {
                    self.lowering_sites.push(new!(LoweringSiteRecord {
                        id: record_id("lowering-construction", &source),
                        source,
                        function_id: self.function_id.to_owned(),
                        operation: format!("{name}!"),
                        constructed_type: Some(constructed_type),
                    }))
                }
                (Err(error), _) => self.errors.push(error.to_string()),
                (Ok(_), Err(error)) => self.errors.push(error.to_string()),
            }
        }
        if matches!(name.as_str(), "optional_entry" | "nonempty_entry") {
            match serialized_key_macro(node) {
                Ok(key) => match self.source_map.span(node.span()) {
                    Ok(source) => self.serialization.push(new!(SerializationRecord {
                        id: record_id("serialized-key-macro", &source),
                        source,
                        owner_id: self.function_id.to_owned(),
                        kind: SerializationKind::SerializedKey,
                        detail: node.tokens.to_string(),
                        key,
                    })),
                    Err(error) => self.errors.push(error.to_string()),
                },
                Err(error) => self.errors.push(error.to_string()),
            }
        }
        visit::visit_macro(self, node);
    }
}

#[requires(!path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|name| !name.is_empty()))]
fn module_name_from_path(path: &str) -> Result<String> {
    const ROOTS: &[(&str, &str)] = &[
        ("crates/jbotci-semantics/src/", "jbotci_semantics"),
        ("apps/jbotci/src/", "jbotci_cli"),
        ("apps/jbotci-server/src/", "jbotci_server"),
        ("xtask-full/src/", "xtask_full"),
    ];
    let Some(&(prefix, crate_name)) = ROOTS
        .iter()
        .find(|(prefix, _)| path.starts_with(*prefix))
    else {
        bail!("Rust inventory path `{path}` has no registered module root");
    };
    let module = path
        .strip_prefix(prefix)
        .expect("registered module root is an exact prefix")
        .strip_suffix(".rs")
        .with_context(|| format!("Rust inventory path `{path}` does not end in `.rs`"))?;
    if matches!(module, "lib" | "main") {
        Ok(crate_name.to_owned())
    } else {
        Ok(format!("{crate_name}::{}", module.replace('/', "::")))
    }
}

#[requires(true)]
#[ensures(true)]
fn path_attribute(attributes: &[Attribute]) -> Option<String> {
    attributes.iter().find_map(|attribute| {
        if !attribute.path().is_ident("path") {
            return None;
        }
        let Meta::NameValue(name_value) = &attribute.meta else {
            return None;
        };
        let Expr::Lit(ExprLit { lit: Lit::Str(path), .. }) = &name_value.value else {
            return None;
        };
        Some(path.value())
    })
}

#[requires(true)]
#[ensures(true)]
fn serde_string_value(meta: &Meta) -> Option<String> {
    let Meta::NameValue(name_value) = meta else {
        return None;
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = &name_value.value
    else {
        return None;
    };
    Some(value.value())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|name| !name.is_empty()))]
fn macro_constructed_type(node: &syn::Macro) -> Result<String> {
    let expression = syn::parse2::<Expr>(node.tokens.clone())?;
    let path = match expression {
        Expr::Struct(expression) => expression.path,
        Expr::Call(expression) => {
            let Expr::Path(path) = *expression.func else {
                bail!("constructed tuple variant is not named by a Rust path");
            };
            path.path
        }
        Expr::Path(expression) => expression.path,
        _ => bail!("unsupported bityzba construction macro shape"),
    };
    Ok(path.to_token_stream().to_string())
}

#[requires(true)]
#[ensures(true)]
fn literal_string_expression(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) => Some(value.value()),
        Expr::Reference(reference) => literal_string_expression(&reference.expr),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn serialized_key_macro(node: &syn::Macro) -> Result<Option<String>> {
    let arguments = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
        .parse2(node.tokens.clone())?;
    if arguments.len() < 2 {
        bail!("serialized-entry macro invocation has fewer than two arguments");
    }
    Ok(arguments.get(1).and_then(literal_string_expression))
}

#[requires(true)]
#[ensures(true)]
fn terminal_type_name(rust_type: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = rust_type else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

#[requires(!path.is_empty() && !name.is_empty())]
#[ensures(true)]
fn registered_consumer_declaration(
    path: &str,
    name: &str,
) -> Option<RendererParserConsumerKind> {
    REGISTERED_CONSUMER_DECLARATIONS
        .iter()
        .find_map(|(registered_path, registered_name, kind)| {
            (*registered_path == path && *registered_name == name).then_some(*kind)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn extracts_complete_declaration_shapes_contracts_and_serialization() {
        let source = r#"
#[invariant(!value.is_empty())]
#[serde(rename_all = "kebab-case")]
struct Carrier { value: String }

enum Shape {
    Named { item: Carrier },
    Tuple(Carrier),
    Unit,
}

impl serde::Serialize for Carrier {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> { todo!() }
}

#[requires(true)]
#[ensures(true)]
fn private_helper() {}
"#;
        let mut inventory = RustInventory::default();
        extract_rust_file(
            "crates/jbotci-semantics/src/sample.rs",
            source,
            FileClass::RustSemanticModel,
            &mut inventory,
        )
        .expect("representative Rust parses");

        assert_eq!(inventory.types.len(), 2);
        assert_eq!(inventory.variants.len(), 3);
        assert!(
            inventory
                .fields
                .iter()
                .any(|field| field.style == FieldStyle::Named)
        );
        assert!(
            inventory
                .fields
                .iter()
                .any(|field| field.style == FieldStyle::Tuple)
        );
        assert!(
            inventory
                .fields
                .iter()
                .any(|field| field.style == FieldStyle::Unit)
        );
        assert!(
            inventory
                .contracts
                .iter()
                .any(|contract| contract.attribute == "#[invariant(!value.is_empty())]")
        );
        assert!(inventory.serialization.iter().any(|record| {
            record.kind == SerializationKind::CustomSerializeImplementation
        }));
        assert!(
            inventory
                .serialization
                .iter()
                .any(|record| record.kind == SerializationKind::SerdeNaming)
        );
        assert!(
            inventory
                .functions
                .iter()
                .any(|function| function.name == "private_helper")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lowering_macro_sites_are_syntax_positioned() {
        let source = r#"
#[requires(true)]
#[ensures(true)]
fn lower() { let _value = new!(SemanticNode { value: 1 }); }
"#;
        let mut inventory = RustInventory::default();
        extract_rust_file(
            "crates/jbotci-semantics/src/generated_builder/sample.rs",
            source,
            FileClass::RustLowering,
            &mut inventory,
        )
        .expect("representative lowering Rust parses");
        assert_eq!(
            inventory
                .lowering_sites
                .iter()
                .filter(|site| site.operation == "new!")
                .count(),
            1
        );
        assert_eq!(
            inventory
                .lowering_sites
                .iter()
                .find(|site| site.operation == "new!")
                .and_then(|site| site.constructed_type.as_deref()),
            Some("SemanticNode")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn malformed_rust_fails_closed() {
        let mut inventory = RustInventory::default();
        let error = extract_rust_file(
            "crates/jbotci-semantics/src/broken.rs",
            "struct Broken {",
            FileClass::RustSemanticModel,
            &mut inventory,
        )
        .expect_err("malformed Rust must fail extraction");
        assert!(error.to_string().contains("parsing pinned Rust"));
    }
}

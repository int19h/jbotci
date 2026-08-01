//! Lojban semantic model and builder facade.

pub mod completeness;
pub mod facade;
pub mod generated_builder;
pub mod model;
pub mod notation;
pub mod references;

pub use facade::{SemanticBuildOptions, SemanticsError, dictionary_relation_place_count};
pub use generated_builder::{
    build_generated_semantic_graph_with_dictionary as build_semantic_graph_with_dictionary,
    build_generated_semantic_graph_with_dictionary,
    build_generated_semantic_graph_with_dictionary_and_options as build_semantic_graph_with_dictionary_and_options,
    build_generated_semantic_graph_with_dictionary_and_options,
};
pub use model::{
    DomainImport, SEMANTIC_JSON_VERSION, ScopeDependence, SemanticGraph, SemanticObject,
    SemanticObjectId, SemanticReferentId, semantic_graph_object_ids_match_types,
    semantic_graph_references_are_defined, semantic_object_domain_imports_are_valid,
    semantic_object_scope_dependences_are_derived,
};
pub use notation::{
    NotationProfile, SmusniConfig, XML_DECLARED_WAIVERS, XmlOmission, XmlRender, XmlSurface,
    XmlWaiverFamily, render_notation, render_smusni, render_xml, render_xml_with_word_cards,
};

/// The XSD 1.0 schema for the complete SFN-XML tersmu document (the
/// `--format xml` notation), covering the compact form, the WORDS word-card
/// section, and the TYPED-GRAPH fallback form.  Every element and attribute
/// is documented in `xs:annotation/xs:documentation`, making this the
/// machine-queryable reference documentation of the notation; it is exposed
/// over MCP as the `jbotci:///tersmu/sfn-xml-v0.xsd` resource.
pub const SFN_XML_SCHEMA_XSD: &str = include_str!("../resources/sfn-xml-v0.xsd");

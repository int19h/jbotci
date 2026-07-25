//! Lojban semantic model and builder facade.

pub mod completeness;
pub mod facade;
pub mod generated_builder;
pub mod model;
pub mod notation;
pub mod references;
pub mod render;

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
pub use notation::{render_lean3, render_notation, Lean3Config, NotationProfile};
pub use render::{render_tree, render_tree_proj};

//! Lojban semantic model and builder facade.

pub mod builder;
pub mod generated_builder;
pub mod model;
pub mod references;

pub use builder::{SemanticBuildOptions, SemanticsError, dictionary_relation_place_count};
pub use generated_builder::{
    build_generated_semantic_graph_with_dictionary as build_semantic_graph_with_dictionary,
    build_generated_semantic_graph_with_dictionary_and_options as build_semantic_graph_with_dictionary_and_options,
    build_generated_semantic_graph_with_dictionary,
    build_generated_semantic_graph_with_dictionary_and_options,
};
pub use model::{
    SEMANTIC_JSON_VERSION, SemanticGraph, SemanticObject, SemanticObjectId, SemanticReferentId,
    semantic_graph_object_ids_match_types, semantic_graph_references_are_defined,
};

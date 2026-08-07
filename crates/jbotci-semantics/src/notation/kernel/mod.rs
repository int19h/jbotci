//! The typed smusni kernel.
//!
//! The kernel is the semantic core of the notation: a strongly typed value
//! model whose constructors validate through the application kernels in
//! [`apply`], so a constructed value is well-typed by construction rather than
//! by a later acceptance parse. It is deliberately notation-independent —
//! nothing under this module mentions `Datum` or any other serialization — so
//! that the version-0 S-expression printer and any future notation are printers
//! over the same value.

pub mod apply;
pub mod binder;
pub mod content;
pub mod document;
pub mod error;
pub mod intrinsic;
pub mod lexicon;
pub mod performable;
pub mod predicate;
pub mod types;
pub mod value;

pub use binder::{
    Bind, BinderSet, Category, Declaration, Lambda, Let, LetRec, RecursiveDeclaration,
    TypedParameter, free_binders_of,
};
pub use content::{AnswerSelection, BinaryOp, Content, JunctionOp, QuantifierOp, Query};
pub use document::{BinderUses, KernelDocument, audit_document_scope, document_binder_uses};
pub use error::KernelTypeError;
pub use intrinsic::Intrinsic;
pub use performable::{Act, Discourse, Performable, TranscriptEntry};
pub use predicate::{PlaceFill, PredTerm};
pub use value::{CollectionKind, FnValue, Literal, Operand, RefComp, Value};

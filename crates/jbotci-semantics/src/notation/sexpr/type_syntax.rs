//! The version-0 concrete type grammar.
//!
//! Kernel types are notation-independent; this module is the only place that
//! reads and writes them as `Datum`. Specification section 2.2 fixes both
//! directions, so the parser and the printer live side by side here rather than
//! on the kernel types themselves.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

use super::super::kernel::types::{
    Force, LexicalRoot, PlaceCandidates, PlaceLabel, PositiveInteger, RelationRef, Row, RowSlot,
    ScalarKind, SignKind, TypeAtom, TypeExpr, TypeParseError, Variable, labels_are_unique,
    row_slots_are_canonical,
};
use super::datum::{Datum, Integer};

/// Serialize a type with the exact type-position grammar.
#[requires(true)]
#[ensures(true)]
pub fn type_to_datum(value: &TypeExpr) -> Datum {
    match value {
        TypeExpr::Atom(atom) => Datum::atom(atom.as_str()),
        TypeExpr::Referents(inner) => Datum::form("Referents", [type_to_datum(inner)]),
        TypeExpr::Set(inner) => Datum::form("Set", [type_to_datum(inner)]),
        TypeExpr::Group(inner) => Datum::form("Group", [type_to_datum(inner)]),
        TypeExpr::List(inner) => Datum::form("List", [type_to_datum(inner)]),
        TypeExpr::Interval(inner) => Datum::form("Interval", [type_to_datum(inner)]),
        TypeExpr::Tuple(elements) => {
            Datum::form("Tuple", [Datum::list(elements.iter().map(type_to_datum))])
        }
        TypeExpr::Function { parameters, result } => Datum::form(
            "Fn",
            [
                Datum::list(parameters.iter().map(type_to_datum)),
                type_to_datum(result),
            ],
        ),
        TypeExpr::Predicate(row) => Datum::form("PredTerm", [row_to_datum(row)]),
        TypeExpr::ReferenceComputation(inner) => Datum::form("RefComp", [type_to_datum(inner)]),
        TypeExpr::Act(force) => Datum::form("Act", [Datum::atom(force.as_str())]),
        TypeExpr::Query(elements) => {
            Datum::form("Query", [Datum::list(elements.iter().map(type_to_datum))])
        }
        TypeExpr::AnswerSelection(elements) => Datum::form(
            "AnswerSelection",
            [Datum::list(elements.iter().map(type_to_datum))],
        ),
        TypeExpr::GeneralizedQuantifier(inner) => Datum::form("GQ", [type_to_datum(inner)]),
        TypeExpr::Sign(kind) => Datum::form("Sign", [Datum::atom(kind.as_str())]),
        TypeExpr::SignToken(kind) => Datum::form("SignToken", [Datum::atom(kind.as_str())]),
        TypeExpr::PlaceOf {
            relation,
            accepted_type,
            candidates,
        } => {
            let mut values = vec![
                relation_ref_to_datum(relation),
                type_to_datum(accepted_type),
            ];
            if let Some(candidates) = candidates {
                values.push(Datum::list(
                    candidates.as_slice().iter().map(place_label_to_datum),
                ));
            }
            Datum::form("PlaceOf", values)
        }
    }
}

/// Serialize an effective row in type position.
#[requires(true)]
#[ensures(ret.form_head() == Some("Row"))]
pub fn row_to_datum(row: &Row) -> Datum {
    let mut children = row
        .slots()
        .iter()
        .map(|slot| {
            Datum::list([
                place_label_to_datum(&slot.label()),
                type_to_datum(slot.accepted_type()),
            ])
        })
        .collect::<Vec<_>>();
    if row.has_open_numbered_tail() {
        children.push(Datum::atom("Open"));
    }
    Datum::form("Row", children)
}

/// Serialize a relation identity as its canonical syntax.
#[requires(true)]
#[ensures(true)]
pub fn relation_ref_to_datum(value: &RelationRef) -> Datum {
    match value {
        RelationRef::Lexical(root) => Datum::atom(root.as_str()),
        RelationRef::Variable(variable) => Datum::atom(variable.as_str()),
        RelationRef::DropPlace { relation, place } => Datum::form(
            "DropPlace",
            [
                relation_ref_to_datum(relation),
                positive_integer_to_datum(place),
            ],
        ),
        RelationRef::Tanru { modifier, head } => Datum::form(
            "Tanru",
            [relation_ref_to_datum(modifier), relation_ref_to_datum(head)],
        ),
        RelationRef::Scalar { kind, relation } => Datum::form(
            "Scalar",
            [Datum::atom(kind.as_str()), relation_ref_to_datum(relation)],
        ),
    }
}

/// Serialize one row or candidate label.
#[requires(true)]
#[ensures(ret.as_atom() == Some("Eventuality") || ret.as_integer().is_some())]
pub fn place_label_to_datum(label: &PlaceLabel) -> Datum {
    match label {
        PlaceLabel::Numbered(value) => positive_integer_to_datum(value),
        PlaceLabel::Eventuality => Datum::atom("Eventuality"),
    }
}

/// Serialize an exact positive place value.
#[requires(true)]
#[ensures(ret.as_integer().is_some())]
pub fn positive_integer_to_datum(value: &PositiveInteger) -> Datum {
    Datum::Integer(
        Integer::try_new(&value.to_string())
            .expect("positive integers have canonical decimal spellings"),
    )
}

/// Print a variable as its canonical atom datum.
///
/// The kernel type invariant already proves the spelling is a lexical atom, so
/// this conversion cannot fail.
#[requires(true)]
#[ensures(ret.as_atom() == Some(variable.as_str()))]
pub fn variable_to_datum(variable: &Variable) -> Datum {
    Datum::atom(variable.as_str())
}

/// Print a lexical root as its canonical atom datum.
#[requires(true)]
#[ensures(ret.as_atom() == Some(root.as_str()))]
pub fn lexical_root_to_datum(root: &LexicalRoot) -> Datum {
    Datum::atom(root.as_str())
}

/// Parse the complete version-0 type grammar.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_type(datum: &Datum) -> Result<TypeExpr, TypeParseError> {
    if let Some(atom) = datum.as_atom() {
        return TypeAtom::parse(atom)
            .map(TypeExpr::Atom)
            .ok_or_else(|| TypeParseError::new(format!("unknown type atom {atom:?}")));
    }
    let items = datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("type must be an atom or type-constructor form"))?;
    let head = items
        .first()
        .and_then(Datum::as_atom)
        .ok_or_else(|| TypeParseError::new("type constructor must be a symbol"))?;
    match head {
        "Referents" | "Set" | "Group" | "List" | "Interval" | "RefComp" | "GQ" => {
            require_len(items, 2, "unary type constructor")?;
            let inner = Box::new(parse_type(&items[1])?);
            Ok(match head {
                "Referents" => TypeExpr::Referents(inner),
                "Set" => TypeExpr::Set(inner),
                "Group" => TypeExpr::Group(inner),
                "List" => TypeExpr::List(inner),
                "Interval" => TypeExpr::Interval(inner),
                "RefComp" => TypeExpr::ReferenceComputation(inner),
                "GQ" => TypeExpr::GeneralizedQuantifier(inner),
                _ => unreachable!("closed match"),
            })
        }
        "Tuple" | "Query" | "AnswerSelection" => {
            require_len(items, 2, "tuple-parameter type constructor")?;
            let elements = parse_type_list(&items[1])?;
            Ok(match head {
                "Tuple" => TypeExpr::Tuple(elements),
                "Query" => TypeExpr::Query(elements),
                "AnswerSelection" => TypeExpr::AnswerSelection(elements),
                _ => unreachable!("closed match"),
            })
        }
        "Fn" => {
            require_len(items, 3, "Fn")?;
            Ok(TypeExpr::Function {
                parameters: parse_type_list(&items[1])?,
                result: Box::new(parse_type(&items[2])?),
            })
        }
        "PredTerm" => {
            require_len(items, 2, "PredTerm")?;
            Ok(TypeExpr::Predicate(parse_row(&items[1])?))
        }
        "Act" => {
            require_len(items, 2, "Act")?;
            let force = items[1]
                .as_atom()
                .and_then(Force::parse)
                .ok_or_else(|| TypeParseError::new("Act requires a closed force literal"))?;
            Ok(TypeExpr::Act(force))
        }
        "Sign" | "SignToken" => {
            require_len(items, 2, head)?;
            let kind = items[1]
                .as_atom()
                .and_then(SignKind::parse)
                .ok_or_else(|| TypeParseError::new("sign type requires a closed sign kind"))?;
            Ok(if head == "Sign" {
                TypeExpr::Sign(kind)
            } else {
                TypeExpr::SignToken(kind)
            })
        }
        "PlaceOf" => parse_place_of(items),
        _ => Err(TypeParseError::new(format!(
            "unknown type constructor {head:?}"
        ))),
    }
}

/// Parse a parenthesized list of types.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_type_list(datum: &Datum) -> Result<Vec<TypeExpr>, TypeParseError> {
    datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("type parameter tuple must be a list"))?
        .iter()
        .map(parse_type)
        .collect()
}

/// Parse a row with unique, ordered labels and optional final `Open` marker.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_row(datum: &Datum) -> Result<Row, TypeParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("row must be a Row form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Row") {
        return Err(TypeParseError::new("row must begin with Row"));
    }
    let open_numbered_tail = items.last().and_then(Datum::as_atom) == Some("Open");
    let slot_end = items.len() - usize::from(open_numbered_tail);
    let mut slots = Vec::new();
    for slot in &items[1..slot_end] {
        let fields = slot
            .as_list()
            .ok_or_else(|| TypeParseError::new("row slot must be a two-item list"))?;
        require_len(fields, 2, "row slot")?;
        let label = parse_place_label(&fields[0])?;
        slots.push(RowSlot::new(label, parse_type(&fields[1])?));
    }
    if !row_slots_are_canonical(&slots) {
        return Err(TypeParseError::new(
            "row slots must be unique and ordered, with Eventuality last",
        ));
    }
    Ok(Row::new(slots, open_numbered_tail))
}

/// Parse a `PlaceOf` type, including its optional explicit candidate set.
#[requires(items.first().and_then(Datum::as_atom) == Some("PlaceOf"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_place_of(items: &[Datum]) -> Result<TypeExpr, TypeParseError> {
    if !matches!(items.len(), 3 | 4) {
        return Err(TypeParseError::new(
            "PlaceOf requires relation, type, and optional candidates",
        ));
    }
    let candidate_labels = items.get(3).map(parse_label_list).transpose()?;
    if candidate_labels.as_ref().is_some_and(Vec::is_empty) {
        return Err(TypeParseError::new(
            "explicit PlaceOf candidates cannot be empty",
        ));
    }
    if candidate_labels
        .as_ref()
        .is_some_and(|labels| !labels_are_unique(labels))
    {
        return Err(TypeParseError::new("PlaceOf candidates must be unique"));
    }
    let candidates = candidate_labels.map(PlaceCandidates::new);
    Ok(TypeExpr::PlaceOf {
        relation: parse_relation_ref(&items[1])?,
        accepted_type: Box::new(parse_type(&items[2])?),
        candidates,
    })
}

/// Parse one relation reference recursively.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_relation_ref(datum: &Datum) -> Result<RelationRef, TypeParseError> {
    if let Some(atom) = datum.as_atom() {
        if atom.starts_with('$') {
            return Variable::try_new(atom).map(RelationRef::Variable);
        }
        return LexicalRoot::try_new(atom).map(RelationRef::Lexical);
    }
    let items = datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("relation reference has invalid shape"))?;
    let head = items.first().and_then(Datum::as_atom);
    match head {
        Some("DropPlace") => {
            require_len(items, 3, "DropPlace relation reference")?;
            let place = parse_positive_integer(&items[2])?;
            Ok(RelationRef::DropPlace {
                relation: Box::new(parse_relation_ref(&items[1])?),
                place,
            })
        }
        Some("Tanru") => {
            require_len(items, 3, "Tanru relation reference")?;
            Ok(RelationRef::Tanru {
                modifier: Box::new(parse_relation_ref(&items[1])?),
                head: Box::new(parse_relation_ref(&items[2])?),
            })
        }
        Some("Scalar") => {
            require_len(items, 3, "Scalar relation reference")?;
            let kind = items[1]
                .as_atom()
                .and_then(ScalarKind::parse)
                .ok_or_else(|| TypeParseError::new("Scalar requires a closed scalar kind"))?;
            Ok(RelationRef::Scalar {
                kind,
                relation: Box::new(parse_relation_ref(&items[2])?),
            })
        }
        _ => Err(TypeParseError::new("unknown relation-reference form")),
    }
}

/// Parse a place-label list.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_label_list(datum: &Datum) -> Result<Vec<PlaceLabel>, TypeParseError> {
    datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("candidate labels must be a list"))?
        .iter()
        .map(parse_place_label)
        .collect()
}

/// Parse one positive numbered or Eventuality label.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_place_label(datum: &Datum) -> Result<PlaceLabel, TypeParseError> {
    if datum.as_atom() == Some("Eventuality") {
        return Ok(PlaceLabel::Eventuality);
    }
    parse_positive_integer(datum).map(PlaceLabel::Numbered)
}

/// Parse an arbitrarily large positive label value.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_positive_integer(datum: &Datum) -> Result<PositiveInteger, TypeParseError> {
    datum
        .as_integer()
        .ok_or_else(|| TypeParseError::new("place label must be a positive integer"))
        .and_then(PositiveInteger::try_new)
}

/// Require an exact form length.
#[requires(!context.is_empty())]
#[ensures(ret.is_ok() == (items.len() == expected))]
fn require_len(items: &[Datum], expected: usize, context: &str) -> Result<(), TypeParseError> {
    if items.len() != expected {
        return Err(TypeParseError::new(format!(
            "{context} requires {} items, found {}",
            expected,
            items.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::super::datum::parse_document;
    use super::*;

    /// Parse one raw type specimen.
    #[requires(true)]
    #[ensures(true)]
    fn ty(text: &str) -> TypeExpr {
        parse_type(&parse_document(text).expect("raw datum parses")).expect("type parses")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_type_constructor_round_trips_structurally() {
        for atom in TypeAtom::ALL {
            assert_eq!(ty(atom.as_str()), TypeExpr::Atom(atom));
        }
        let specimens = [
            "Entity",
            "(Referents Eventuality)",
            "(Set Entity)",
            "(Group Entity)",
            "(List Number)",
            "(Interval Number)",
            "(Tuple (Entity Number))",
            "(Fn (Entity Number) Content)",
            "(PredTerm (Row (1 (Referents Entity)) (Eventuality (Referents Eventuality)) Open))",
            "(RefComp (Referents Entity))",
            "(Act Assertion)",
            "(Query ())",
            "(AnswerSelection (Entity Number))",
            "(GQ Entity)",
            "(Sign Sentence)",
            "(SignToken Structured)",
            "(PlaceOf (DropPlace klama 3) (Referents Entity) (1 2 Eventuality))",
            "(PredTerm (Row (429496729600000000000000000000 Entity)))",
            "(PlaceOf (DropPlace klama 429496729600000000000000000000) Entity)",
        ];
        for specimen in specimens {
            let parsed = ty(specimen);
            assert_eq!(parse_type(&type_to_datum(&parsed)), Ok(parsed));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn malformed_type_row_and_relation_forms_are_rejected() {
        for specimen in [
            "BogusType",
            "(BogusType Entity)",
            "(Referents)",
            "(Act Opaque)",
            "(Sign BogusKind)",
            "(PredTerm NotARow)",
            "(PredTerm (Row (2 Entity) (1 Entity)))",
            "(PredTerm (Row (1 Entity) (1 Entity)))",
            "(PredTerm (Row (Eventuality Eventuality) (1 Entity)))",
            "(PredTerm (Row Open (1 Entity)))",
            "(PlaceOf klama Entity ())",
            "(PlaceOf klama Entity (1 1))",
            "(PlaceOf Klama Entity)",
            "(PlaceOf (DropPlace klama 0) Entity)",
            "(PlaceOf (Scalar Bogus klama) Entity)",
            "(PlaceOf (Tanru klama) Entity)",
        ] {
            let datum = parse_document(specimen).expect("malformed type specimen is lexical");
            assert!(
                parse_type(&datum).is_err(),
                "accepted malformed type {specimen}",
            );
        }
    }
}

//! Static completeness coverage for the ordinary SFN-XML profile.
//!
//! This inventory contract is intentionally paired with, not substituted for,
//! the occurrence-level omission accounting in [`super::render_xml`]. The
//! contract proves every authored semantic surface has a disposition; the
//! frozen 48-document tests prove that every known compact surface avoids the
//! generic fallback and that observed omissions are exactly the seven
//! owner-audited provenance families. `TYPED-GRAPH` is a deliberately separate
//! whole-document projection and is not part of this compact-form assertion.

#[allow(unused_imports)]
use bityzba::{data, ensures, requires};

use crate::completeness::model::DispositionData;
use crate::completeness::{
    CompletenessContract, Disposition, InventoryEntry, render_field_inventory, source_link_surfaces,
};

const SOURCE_REASON: &str = "source provenance; SFN-XML renders semantic content, not source spans, witness text, or construct labels";
const ASSIGNED_NAME_REASON: &str =
    "assigned-name provenance; ordinary SFN-XML omits assigned-name records";
const DESCRIPTOR_WORD_REASON: &str =
    "descriptor surface-word provenance; SFN-XML renders descriptor semantics";
const INTRODUCED_BY_REASON: &str =
    "surface introducer provenance; ordinary SFN-XML omits introducedBy fields";
const QUANTITY_TEXT_REASON: &str =
    "quantity surface-text provenance; SFN-XML renders the semantic quantity form";
const COMPOSITION_RELATION_LABEL_REASON: &str = "composition relation-label provenance; ordinary SFN-XML preserves the modifier/kind structure while provenance mode alone renders the derivational label";

#[requires(true)]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))))]
fn is_source_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))
}

#[requires(true)]
#[ensures(true)]
fn waiver_reason(entry: &InventoryEntry) -> Option<&'static str> {
    if is_source_provenance(entry) {
        return Some(SOURCE_REASON);
    }
    if entry.field == "introducedBy" {
        return Some(INTRODUCED_BY_REASON);
    }
    if entry.surface.name == "AssignedName"
        || matches!(entry.surface.name, "Referent" | "Eventuality")
            && entry.field == "assignedNames"
    {
        return Some(ASSIGNED_NAME_REASON);
    }
    if entry.surface.name == "Descriptor" && entry.field == "word" {
        // This field also carries the separately audited bound-variable word
        // family at the occurrence level.
        return Some(DESCRIPTOR_WORD_REASON);
    }
    if entry.surface.name == "QuantityValue" && entry.field == "text" {
        return Some(QUANTITY_TEXT_REASON);
    }
    if entry.surface.name == "TanruLink" && entry.field == "relationLabel" {
        return Some(COMPOSITION_RELATION_LABEL_REASON);
    }
    None
}

/// The ordinary XML renderer's disposition for one completeness entry.
///
/// #709 two-mode note: `Predication:relationMetadata` (and the entire
/// `RelationMetadata` object subtree it references) is `Renders` in both
/// document shapes — never a waiver. With a WORDS word-card section present,
/// the decomposition is rendered via the nonce word's WORD card and body
/// predications carry no `RELATION-METADATA` element (rendered-via-card, with
/// no omission entries); without cards, the interim body `RELATION-METADATA`
/// preservation form is retained deliberately so the decomposition is never
/// silently dropped.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_)))
    == waiver_reason(entry).is_some())]
#[ensures(matches!(ret.as_data(), data!(Disposition::Renders))
    == waiver_reason(entry).is_none())]
pub fn xml_renderer_disposition(entry: &InventoryEntry) -> Disposition {
    waiver_reason(entry).map_or_else(Disposition::renders, Disposition::excluded_with_reason)
}

/// A complete disposition map for the ordinary SFN-XML profile.
#[requires(true)]
#[ensures(ret.len() == render_field_inventory().len())]
pub fn xml_coverage_contract() -> CompletenessContract {
    let inventory = render_field_inventory();
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract
            .try_register(entry.key(), xml_renderer_disposition(entry))
            .expect("inventory entries are unique by key");
    }
    contract
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xml_coverage_audits_complete() {
        let inventory = render_field_inventory();
        let audit = xml_coverage_contract().audit(&inventory);
        assert!(
            audit.is_complete(),
            "missing={:?}, orphans={:?}",
            audit.missing,
            audit.orphans
        );
    }
}

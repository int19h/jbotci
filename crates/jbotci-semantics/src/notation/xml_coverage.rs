//! Static completeness coverage for the ordinary SFN-XML profile.
//!
//! This inventory contract is intentionally paired with, not substituted for,
//! the occurrence-level omission accounting in [`super::render_xml`]. The
//! contract proves every authored semantic surface has a disposition; the
//! frozen 46-document tests prove the renderer's observed omissions are exactly
//! the six owner-audited provenance families.

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
    None
}

/// The ordinary XML renderer's disposition for one completeness entry.
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

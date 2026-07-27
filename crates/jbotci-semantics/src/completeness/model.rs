//! Typed model for the render-field completeness inventory and its contract.
//!
//! Phase-B step 2 of the tersmu notation program (see the research repo's
//! `DESIGN-RECORD.md`: "COMPLETENESS INVENTORY + CONTRACT before renderer
//! code"). This module carries the *types*; [`super::inventory`] carries the
//! authored entries and [`super::disposition`] the baseline contract.
//!
//! The inventory enumerates every serializable field, every enum variant, and a
//! curated set of derived/conditional facts of the `lojban-semantics-json-1`
//! surface (the [`crate::model`] types serialized by `tersmu --format json`).
//! Serde-schema enumeration alone is insufficient — derived facts and
//! variant-conditional shapes need explicit entries — so entries are typed by
//! [`EntryKind`] and carry an explicit [`Presence`] condition and a [`Witness`]
//! pointer into the frozen corpus.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use std::collections::BTreeMap;

/// Which category of the JSON surface an entry belongs to.
///
/// The category tells a reader (and the drift-guard test) how to locate the
/// entry's Rust definition: an `Object` is one of the top-level
/// [`crate::model::SemanticObject`] node kinds, a `ValueStruct` is a nested
/// value type serialized as a JSON object, an `Enum` carries variant
/// discriminants, and `Document` covers document-level structural facts fixed
/// by `FREEZE-PHASE-B.md`.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SurfaceCategory {
    Object,
    ValueStruct,
    Enum,
    Document,
}

/// A named region of the serialized surface: a Rust object kind, value struct,
/// enum, or the document envelope.
///
/// `name` is the Rust type name (or `"document"` for the envelope). It is the
/// stable identity a future renderer registers coverage against, so it must be
/// non-empty.
#[invariant(!name.is_empty(), "a surface must name its Rust type")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Surface {
    pub category: SurfaceCategory,
    pub name: &'static str,
}

impl Surface {
    #[requires(!name.is_empty())]
    #[ensures(ret.name == name && ret.category == category)]
    pub fn new(category: SurfaceCategory, name: &'static str) -> Self {
        new!(Surface { category, name })
    }
}

/// What kind of inventory item an entry records.
///
/// A `Field` is a serializable field of an object or value struct; a `Variant`
/// is an enum discriminant (a variant-conditional branch a renderer must
/// handle); a `DerivedFact` is a rendered fact that is *not* a single serde
/// field — e.g. the reference sort header derived from a referent's `sort`, or
/// a `NOT COMPUTED` declaration.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryKind {
    Field,
    Variant,
    DerivedFact,
}

/// The presence condition of an inventoried item in a serialized document.
///
/// This is deliberately distinct from a renderer [`Disposition`]: presence is a
/// property of the *data* (when the field/variant appears), while a disposition
/// is a property of the *renderer* (what it does with the item). In particular
/// an [`Presence::Optional`] field that is absent from a given document is a
/// legitimate absence, never a spurious "not computed".
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Presence {
    /// Emitted for every object of this surface's kind (unconditional).
    Always,
    /// Emitted only when populated; `skip_serializing_if` elides it otherwise.
    /// Absence is legitimate and document-specific, not a missing computation.
    Optional,
    /// Emitted only for a subset of the surface's variants/shapes (e.g. a field
    /// present only for one `FormulaNode` variant, or one enum variant).
    VariantConditional,
    /// A field of a nested value struct; present exactly when its parent
    /// container is present (inherits the parent's optionality).
    Nested,
    /// A computed fact with no single serde field of its own.
    Derived,
}

/// Whether a corpus witness asserts mere presence or a specific scalar value.
#[invariant(::Present => true)]
#[invariant(::Value(value) => !value.is_empty())]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessExpect {
    /// The JSON coordinate is populated with a non-null value in the witness
    /// document. The corpus walker treats a JSON `null` as absent (it is never
    /// recorded as a coordinate), so `Present` means present-and-non-null; the
    /// model elides absent optionals via `skip_serializing_if` rather than
    /// emitting `null`, so the two coincide in practice.
    Present,
    /// The JSON coordinate carries this exact scalar value (variant witness).
    Value(&'static str),
}

/// A pointer to where an inventoried item is exercised in the frozen corpus.
///
/// [`Witness::Corpus`] names a corpus document (by its frozen file stem) and a
/// concrete JSON coordinate re-derivable by *this* build; the corpus-witness
/// test verifies it. [`Witness::NoCorpusWitness`] marks an item the frozen
/// corpus does not exercise — future test-fixture debt, listed rather than
/// silently absent.
#[invariant(::Corpus { doc, path, .. } => !doc.is_empty() && !path.is_empty())]
#[invariant(::NoCorpusWitness => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Witness {
    Corpus {
        doc: &'static str,
        /// `"<NodeType>:<dotted.path>"`; `x*` stands for any numbered place and
        /// list elements collapse onto their element path.
        path: &'static str,
        expect: WitnessExpect,
    },
    NoCorpusWitness,
}

impl Witness {
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(Witness::Corpus { .. })))]
    pub fn is_witnessed(&self) -> bool {
        matches!(self.as_data(), data!(Witness::Corpus { .. }))
    }

    /// The `(doc, path, expect)` of a corpus witness, if any.
    #[requires(true)]
    #[ensures(ret.is_some() == self.is_witnessed())]
    pub fn corpus(&self) -> Option<(&'static str, &'static str, WitnessExpect)> {
        match self.as_data() {
            data!(Witness::Corpus { doc, path, expect }) => Some((*doc, *path, *expect)),
            data!(Witness::NoCorpusWitness) => None,
        }
    }
}

impl WitnessExpect {
    /// The exact scalar value a variant witness expects, if any.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(WitnessExpect::Value(_))))]
    pub fn value(&self) -> Option<&'static str> {
        match self.as_data() {
            data!(WitnessExpect::Value(value)) => Some(*value),
            data!(WitnessExpect::Present) => None,
        }
    }
}

/// One inventory entry: a serializable field, an enum variant, or a derived
/// fact of the semantic surface.
///
/// `(surface, field, kind)` is the entry's identity; the completeness contract
/// keys dispositions on it. `variant_of`, when present, names the enum-variant
/// or node-variant shape that gates a `VariantConditional` field, so the reader
/// sees *which* branch introduces the field.
#[invariant(!field.is_empty(), "an entry must name its field/variant/fact")]
#[invariant((*kind == EntryKind::Field) || (*presence != Presence::Always) || (surface.category != SurfaceCategory::Enum),
    "enum surfaces carry variants or derived facts, not unconditional fields")]
#[invariant(variant_of.is_none_or(|shape| !shape.is_empty()), "a named gating shape must be non-empty")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryEntry {
    pub surface: Surface,
    pub field: &'static str,
    pub kind: EntryKind,
    pub presence: Presence,
    pub witness: Witness,
    pub variant_of: Option<&'static str>,
}

impl InventoryEntry {
    #[requires(!field.is_empty())]
    #[requires(variant_of.is_none_or(|shape| !shape.is_empty()))]
    #[ensures(ret.field == field && ret.surface == surface)]
    pub fn new(
        surface: Surface,
        field: &'static str,
        kind: EntryKind,
        presence: Presence,
        witness: Witness,
        variant_of: Option<&'static str>,
    ) -> Self {
        new!(InventoryEntry {
            surface,
            field,
            kind,
            presence,
            witness,
            variant_of,
        })
    }

    /// The identity key used to map an entry to a renderer disposition.
    #[requires(true)]
    #[ensures(ret.surface == self.surface.name && ret.field == self.field && ret.kind == self.kind)]
    pub fn key(&self) -> EntryKey {
        new!(EntryKey {
            category: self.surface.category,
            surface: self.surface.name,
            field: self.field,
            kind: self.kind,
        })
    }
}

/// The identity of an inventory entry, used as the disposition map key.
#[invariant(!surface.is_empty() && !field.is_empty())]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryKey {
    pub category: SurfaceCategory,
    pub surface: &'static str,
    pub field: &'static str,
    pub kind: EntryKind,
}

/// The full render-field inventory: a de-duplicated, ordered set of entries.
///
/// Constructed only through [`RenderFieldInventory::new`], which enforces that
/// entry identities are unique — a duplicated field/variant would let a renderer
/// register one disposition and silently leave another uncovered.
#[invariant(
    entries_have_unique_keys(entries),
    "inventory entries must have unique identities"
)]
#[derive(Debug, Clone, PartialEq)]
pub struct RenderFieldInventory {
    entries: Vec<InventoryEntry>,
}

impl RenderFieldInventory {
    #[requires(entries_have_unique_keys(&entries))]
    #[ensures(ret.len() == old(entries.len()))]
    pub fn new(entries: Vec<InventoryEntry>) -> Self {
        new!(RenderFieldInventory { entries })
    }

    #[requires(true)]
    #[ensures(ret == self.entries.len())]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[requires(true)]
    #[ensures(ret == self.entries.is_empty())]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }

    /// Count of entries with a concrete corpus witness.
    #[requires(true)]
    #[ensures(ret <= self.entries.len())]
    pub fn witnessed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.witness.is_witnessed())
            .count()
    }
}

/// How a renderer treats an inventoried item.
///
/// `Renders` — the renderer emits it. `NotComputedDeclared` — the renderer
/// declares it explicitly uncomputed (a `NOT COMPUTED` marker), *not* the same
/// as an absent optional. `ExcludedWithReason` — deliberately omitted, with a
/// recorded reason so the omission is auditable rather than accidental.
#[invariant(::Renders => true)]
#[invariant(::NotComputedDeclared => true)]
#[invariant(::ExcludedWithReason(reason) => !reason.is_empty())]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Renders,
    NotComputedDeclared,
    ExcludedWithReason(&'static str),
}

impl Disposition {
    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(Disposition::Renders)))]
    pub fn renders() -> Self {
        new!(Disposition::Renders)
    }

    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(Disposition::NotComputedDeclared)))]
    pub fn not_computed_declared() -> Self {
        new!(Disposition::NotComputedDeclared)
    }

    #[requires(!reason.is_empty())]
    #[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_))))]
    pub fn excluded_with_reason(reason: &'static str) -> Self {
        new!(Disposition::ExcludedWithReason(reason))
    }
}

/// A checked mapping from inventory entries to renderer dispositions.
///
/// A future renderer registers its coverage here; [`CompletenessContract::audit`]
/// against a [`RenderFieldInventory`] fails if any inventoried entry lacks a
/// disposition or any registered disposition names an entry the inventory does
/// not contain (an orphan).
#[invariant(true)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompletenessContract {
    dispositions: BTreeMap<EntryKey, Disposition>,
}

/// The outcome of auditing a contract against an inventory.
#[invariant(missing.iter().all(|key| !key.field.is_empty()))]
#[invariant(orphans.iter().all(|key| !key.field.is_empty()))]
#[derive(Debug, Clone, PartialEq)]
pub struct ContractAudit {
    /// Inventory entries with no registered disposition.
    pub missing: Vec<EntryKey>,
    /// Registered dispositions that name no inventory entry.
    pub orphans: Vec<EntryKey>,
}

impl ContractAudit {
    #[requires(true)]
    #[ensures(ret == (self.missing.is_empty() && self.orphans.is_empty()))]
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty() && self.orphans.is_empty()
    }
}

impl CompletenessContract {
    #[requires(true)]
    #[ensures(ret.dispositions.is_empty())]
    pub fn new() -> Self {
        // `#[invariant(true)]` is an audited no-op marker, so this type keeps a
        // plain literal constructor rather than a generated data API.
        CompletenessContract {
            dispositions: BTreeMap::new(),
        }
    }

    /// Register (or overwrite) the disposition for an entry key.
    ///
    /// The design-intent baseline builds a contract from scratch, so silent
    /// overwrite is harmless here; a renderer accumulating coverage should use
    /// [`CompletenessContract::try_register`] instead, which rejects a
    /// double-registration rather than masking it.
    #[requires(true)]
    #[ensures(self.dispositions.get(&key) == Some(&disposition))]
    pub fn register(&mut self, key: EntryKey, disposition: Disposition) {
        self.dispositions.insert(key, disposition);
    }

    /// Register a disposition, failing (returning the key) if one is already
    /// registered — so a renderer cannot silently double-cover an entry.
    #[requires(true)]
    #[ensures(ret.is_ok() == old(self.dispositions.get(&key).is_none()))]
    #[ensures(ret.is_ok() -> (self.dispositions.get(&key) == Some(&disposition)))]
    pub fn try_register(
        &mut self,
        key: EntryKey,
        disposition: Disposition,
    ) -> Result<(), EntryKey> {
        if self.dispositions.contains_key(&key) {
            return Err(key);
        }
        self.dispositions.insert(key, disposition);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret == self.dispositions.get(&key))]
    pub fn disposition_for(&self, key: &EntryKey) -> Option<&Disposition> {
        self.dispositions.get(key)
    }

    /// Iterate the registered `(key, disposition)` pairs in key order — the
    /// renderer PR consumes this to drive rendering from the contract.
    #[requires(true)]
    #[ensures(true)]
    pub fn iter(&self) -> impl Iterator<Item = (&EntryKey, &Disposition)> {
        self.dispositions.iter()
    }

    #[requires(true)]
    #[ensures(ret == self.dispositions.len())]
    pub fn len(&self) -> usize {
        self.dispositions.len()
    }

    #[requires(true)]
    #[ensures(ret == self.dispositions.is_empty())]
    pub fn is_empty(&self) -> bool {
        self.dispositions.is_empty()
    }

    /// Audit this contract against an inventory: every entry must be
    /// dispositioned, and every disposition must name a real entry.
    #[requires(true)]
    #[ensures(ret.is_complete() == (ret.missing.is_empty() && ret.orphans.is_empty()))]
    #[ensures(ret.is_complete() -> (self.dispositions.len() >= inventory.len() || inventory.is_empty()))]
    pub fn audit(&self, inventory: &RenderFieldInventory) -> ContractAudit {
        let mut inventory_keys = std::collections::BTreeSet::new();
        let mut missing = Vec::new();
        for entry in inventory.entries() {
            let key = entry.key();
            inventory_keys.insert(key);
            if self.dispositions.get(&key).is_none() {
                missing.push(key);
            }
        }
        let orphans = self
            .dispositions
            .keys()
            .filter(|key| !inventory_keys.contains(*key))
            .copied()
            .collect();
        new!(ContractAudit { missing, orphans })
    }

    /// Entry keys where this contract's disposition differs from `expected`'s —
    /// the expected-vs-implemented mismatch check a renderer's registered
    /// coverage is held to against the design-intent baseline. Only keys present
    /// in both contracts are compared; presence gaps are the [`Self::audit`]'s job.
    #[requires(true)]
    #[ensures(ret.iter().all(|key| self.dispositions.get(key) != expected.dispositions.get(key)))]
    pub fn disagreements(&self, expected: &CompletenessContract) -> Vec<EntryKey> {
        self.dispositions
            .iter()
            .filter(|(key, disposition)| {
                expected
                    .dispositions
                    .get(*key)
                    .is_some_and(|other| other != *disposition)
            })
            .map(|(key, _)| *key)
            .collect()
    }
}

/// True when no two entries share `(category, surface, field, kind)`.
#[requires(true)]
#[ensures(true)]
pub fn entries_have_unique_keys(entries: &[InventoryEntry]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    entries.iter().all(|entry| seen.insert(entry.key()))
}

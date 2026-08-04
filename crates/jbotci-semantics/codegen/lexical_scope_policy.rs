//! Offline generator and audit for the curated lexical relation/place policy.
//!
//! This module is shared by `build.rs` and rejection tests. It deliberately
//! reads only caller-supplied bytes; there is no network client or prose-based
//! place inference anywhere in the generation path.

#![allow(dead_code)] // `build.rs` and the rejection-test crate use complementary APIs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const SOURCE_URL: &str =
    "https://www.lojban.org/static/publications/wordlists/oblique_keywords.txt";
const SOURCE_SHA256: &str = "355786cfd049063c92514fac2d417fc4966df7749dc17d7cfb49bd903fb6a2cb";
const SOURCE_HTTP_LAST_MODIFIED: &str = "Tue, 28 Jun 2005 04:50:44 GMT";
const SOURCE_RETRIEVED_AT: &str = "2026-08-04T06:29:59Z";
const SOURCE_BYTE_COUNT: usize = 79_293;
const SOURCE_RECORD_COUNT: usize = 3_542;
const SOURCE_ROOT_COUNT: usize = 1_347;
const SOURCE_ROOTS_WITH_X1: usize = 1_346;
const DICTIONARY_SHA256: &str = "ba268ad701f8f44656ea4b17a1fd9539cfc1a3c523d0bdf581a44e3e93bb412f";
const DICTIONARY_CREATED_AT: &str = "2026-07-27T07:10:51.776063Z";
const DICTIONARY_ENTRY_COUNT: usize = 17_536;
const DICTIONARY_GISMU_COUNT: usize = 1_338;
const WITNESS_SHA256: &str = "850e960656599617673acbbbad5fe0384c681f8ab4587aa234b6c82e9478106b";

const WITNESSES: &str = concat!(
    "mi klama\n",
    "mi klama fu lo karce\n",
    "mi dunda zi'o ti\n",
    "mi klama sepi'o lo karce\n",
    "mi klama fi'o pilno lo karce\n",
    "ro mlatu cu jbena\n",
    "ti blanu zdani\n",
    "mi pu klama lo zarci\n",
    "mi djica lo nu mi cilre\n",
    "lo ci gerku cu blabi\n",
    "le gerku poi blabi cu melbi\n",
    "lo cukta noi mi nelci ke'a cu melbi\n",
    "ci gerku ce'e re nanmu cu batci\n",
    "ma klama lo zarci\n",
    "ti mo\n",
    "xu do klama\n",
    "li re su'i ci du li mu\n",
    "ni'o mi klama\n",
);

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenErrorKind {
    Io,
    Parse,
    SourceMetadata,
    WholeSourceAudit,
    LexicalIdentity,
    Fingerprint,
    KeyRange,
    Duplicate,
    ClosedEnum,
    Evidence,
    Coverage,
    NonDeterministicOrder,
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub kind: CodegenErrorKind,
    pub message: String,
}

impl CodegenError {
    #[requires(true)]
    #[ensures(ret.kind == kind && !ret.message.is_empty())]
    fn new(kind: CodegenErrorKind, message: impl Into<String>) -> Self {
        let message = message.into();
        new!(CodegenError { kind, message })
    }
}

impl fmt::Display for CodegenError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "lexical policy {:?}: {}",
            self.kind, self.message
        )
    }
}

impl std::error::Error for CodegenError {}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorPaths {
    pub source: PathBuf,
    pub source_metadata: PathBuf,
    pub policies: PathBuf,
    pub witnesses: PathBuf,
    pub dictionary: PathBuf,
    pub dictionary_metadata: PathBuf,
    pub output: PathBuf,
}

impl GeneratorPaths {
    #[requires(true)]
    #[ensures(ret.len() == 6)]
    pub fn inputs(&self) -> [&Path; 6] {
        [
            &self.source,
            &self.source_metadata,
            &self.policies,
            &self.witnesses,
            &self.dictionary,
            &self.dictionary_metadata,
        ]
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct GeneratorInputs<'a> {
    pub source: &'a [u8],
    pub source_metadata: &'a str,
    pub policies: &'a str,
    pub witnesses: &'a [u8],
    pub dictionary: &'a [u8],
    pub dictionary_metadata: &'a str,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct SourceMetadata {
    url: String,
    sha256: String,
    http_last_modified: String,
    retrieved_at: String,
    byte_count: usize,
    record_count: usize,
    source_root_count: usize,
    source_roots_with_x1: usize,
    lensisku_sha256: String,
    lensisku_created_at: String,
    lensisku_entry_count: usize,
    lensisku_gismu_count: usize,
    lensisku_gismu_with_x1: usize,
    malformed_key_count: usize,
    duplicate_key_count: usize,
    observed_gaps: Vec<String>,
    mandatory_arities: BTreeMap<String, usize>,
    curated_root: Vec<CuratedRoot>,
}

#[invariant(true)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct CuratedRoot {
    root: String,
    line_start: usize,
    line_end: usize,
    arity: usize,
    status: String,
    definition_id: u64,
    lexical_fingerprint: String,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct DictionaryMetadata {
    lensisku_created_at: String,
    sha256: String,
    entry_count: usize,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct PolicyFile {
    relation_place: Vec<RelationPlace>,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct RelationPlace {
    relation: String,
    word_type: String,
    definition_id: u64,
    attested_arity: usize,
    original_place: usize,
    lexical_fingerprint: String,
    place_key_source: String,
    identity_evidence: Vec<String>,
    family_policy: Vec<FamilyPolicy>,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct FamilyPolicy {
    dynamic_family: String,
    policy: String,
    family_evidence: Vec<String>,
    policy_evidence: Vec<String>,
    negative_evidence: Vec<String>,
    coverage: Vec<String>,
    rationale: String,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct RootAudit {
    places: BTreeSet<usize>,
    lines: Vec<usize>,
}

#[invariant(true)]
#[derive(Debug)]
struct SourceAudit {
    roots: BTreeMap<String, RootAudit>,
    metadata: SourceMetadata,
}

#[invariant(!relation.is_empty() && *original_place > 0 && *attested_arity >= *original_place)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedRow {
    relation: String,
    original_place: usize,
    attested_arity: usize,
    family_variant: &'static str,
    policy_variant: &'static str,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct ExpectedCurated {
    root: &'static str,
    line_start: usize,
    line_end: usize,
    arity: usize,
    status: &'static str,
    definition_id: u64,
    fingerprint: &'static str,
}

const CURATED: &[ExpectedCurated] = &[
    ExpectedCurated {
        root: "blabi",
        line_start: 153,
        line_end: 153,
        arity: 1,
        status: "initial",
        definition_id: 58,
        fingerprint: "8380a7326db25a3a6dd38423a03eb56ec70b3f54d1f04c27b85e36caeb90292d",
    },
    ExpectedCurated {
        root: "djica",
        line_start: 805,
        line_end: 807,
        arity: 3,
        status: "initial",
        definition_id: 299,
        fingerprint: "d1ba2221c03c09da12995b3007425e22998d6610f2e7c432126e00674e5724c6",
    },
    ExpectedCurated {
        root: "klama",
        line_start: 1578,
        line_end: 1582,
        arity: 5,
        status: "initial",
        definition_id: 583,
        fingerprint: "2854acc5a6bd0cba55540a15310961d5cba8387561e411997713f6be9190615c",
    },
    ExpectedCurated {
        root: "melbi",
        line_start: 1915,
        line_end: 1918,
        arity: 4,
        status: "initial",
        definition_id: 714,
        fingerprint: "8cfa4751d2ac12b4a5430771d184a9e6927fa6e646956fd43d794fe5888557c0",
    },
    ExpectedCurated {
        root: "nitcu",
        line_start: 2138,
        line_end: 2140,
        arity: 3,
        status: "verified-future",
        definition_id: 795,
        fingerprint: "5eb7fa2ba3085e8fe23d8d3a95aae476a7a085971ef1a34e7870ae5e2404a965",
    },
    ExpectedCurated {
        root: "pilno",
        line_start: 2278,
        line_end: 2280,
        arity: 3,
        status: "initial",
        definition_id: 847,
        fingerprint: "17cedcb159ce898698c8fbc4586027f0960973bcda4935a2e8fa3528211b6732",
    },
    ExpectedCurated {
        root: "sisku",
        line_start: 2727,
        line_end: 2729,
        arity: 3,
        status: "withheld",
        definition_id: 1017,
        fingerprint: "1c78c09c4bbeeadddd16b94d03cad7464861885c1b99c9321383f6e74e1fc994",
    },
    ExpectedCurated {
        root: "troci",
        line_start: 3166,
        line_end: 3168,
        arity: 3,
        status: "withheld",
        definition_id: 1191,
        fingerprint: "e37ac95b5760329c084e63d1cc9a9a5af5baf4a17f67fdd763f135bc23bc0cf1",
    },
];

const MANDATORY_ARITIES: &[(&str, usize)] = &[
    ("batci", 4),
    ("blabi", 1),
    ("blanu", 1),
    ("cilre", 5),
    ("cukta", 5),
    ("djica", 3),
    ("dunda", 3),
    ("gerku", 2),
    ("jbena", 4),
    ("karce", 3),
    ("klama", 5),
    ("melbi", 4),
    ("mlatu", 2),
    ("nanmu", 1),
    ("nelci", 2),
    ("pilno", 3),
    ("purci", 2),
    ("skicu", 4),
    ("zarci", 3),
    ("zdani", 2),
];

const EXPECTED_POLICY_KEYS: &[(&str, usize, &str, &[usize])] = &[
    ("blabi", 1, "ref-comp-referents-entity", &[10]),
    ("djica", 2, "ref-comp-referents-eventuality", &[9]),
    ("klama", 2, "ref-comp-referents-entity", &[8, 14]),
    ("klama", 5, "ref-comp-referents-entity", &[2]),
    ("melbi", 1, "ref-comp-referents-entity", &[11, 12]),
    ("pilno", 1, "ref-comp-referents-entity", &[5]),
    ("pilno", 2, "ref-comp-referents-entity", &[4]),
];

#[requires(true)]
#[ensures(ret.len() == 64)]
fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[requires(true)]
#[ensures(ret == (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_lowercase())))]
fn is_canonical_relation(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_lowercase())
}

#[requires(true)]
#[ensures(ret.is_ok() -> !ret.as_ref().unwrap().0.is_empty())]
fn parse_place_key(key: &str) -> Result<(String, usize), CodegenError> {
    let split = key
        .find(|character: char| character.is_ascii_digit())
        .ok_or_else(|| {
            CodegenError::new(
                CodegenErrorKind::WholeSourceAudit,
                format!("place key has no numeric suffix: {key}"),
            )
        })?;
    let (root, digits) = key.split_at(split);
    if !is_canonical_relation(root)
        || digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(CodegenError::new(
            CodegenErrorKind::WholeSourceAudit,
            format!("malformed place key: {key}"),
        ));
    }
    let place = digits.parse::<usize>().map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::WholeSourceAudit,
            format!("invalid place number in {key}: {error}"),
        )
    })?;
    if place == 0 {
        return Err(CodegenError::new(
            CodegenErrorKind::KeyRange,
            format!("place key is zero: {key}"),
        ));
    }
    Ok((root.to_owned(), place))
}

#[requires(true)]
#[ensures(ret.is_ok() -> ret.as_ref().unwrap().roots.len() == SOURCE_ROOT_COUNT)]
fn audit_source(bytes: &[u8], metadata_text: &str) -> Result<SourceAudit, CodegenError> {
    let metadata: SourceMetadata = toml::from_str(metadata_text).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("parse source metadata: {error}"),
        )
    })?;
    let metadata_matches = metadata.url == SOURCE_URL
        && metadata.sha256 == SOURCE_SHA256
        && metadata.http_last_modified == SOURCE_HTTP_LAST_MODIFIED
        && metadata.retrieved_at == SOURCE_RETRIEVED_AT
        && metadata.byte_count == SOURCE_BYTE_COUNT
        && metadata.record_count == SOURCE_RECORD_COUNT
        && metadata.source_root_count == SOURCE_ROOT_COUNT
        && metadata.source_roots_with_x1 == SOURCE_ROOTS_WITH_X1
        && metadata.lensisku_sha256 == DICTIONARY_SHA256
        && metadata.lensisku_created_at == DICTIONARY_CREATED_AT
        && metadata.lensisku_entry_count == DICTIONARY_ENTRY_COUNT
        && metadata.lensisku_gismu_count == DICTIONARY_GISMU_COUNT
        && metadata.lensisku_gismu_with_x1 == DICTIONARY_GISMU_COUNT
        && metadata.malformed_key_count == 0
        && metadata.duplicate_key_count == 0
        && metadata.observed_gaps == ["moi1", "molki3"];
    if !metadata_matches {
        return Err(CodegenError::new(
            CodegenErrorKind::SourceMetadata,
            "source sidecar differs from the reviewed whole-file contract",
        ));
    }
    if bytes.len() != SOURCE_BYTE_COUNT {
        return Err(CodegenError::new(
            CodegenErrorKind::SourceMetadata,
            "vendored source byte count differs",
        ));
    }
    let crlf_is_exact = bytes.ends_with(b"\r\n")
        && bytes.iter().enumerate().all(|(index, byte)| match *byte {
            b'\n' => index > 0 && bytes[index - 1] == b'\r',
            b'\r' => bytes.get(index + 1) == Some(&b'\n'),
            _ => true,
        });
    if !crlf_is_exact {
        return Err(CodegenError::new(
            CodegenErrorKind::SourceMetadata,
            "vendored source is not exact CRLF records",
        ));
    }
    let records: Vec<&[u8]> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|record| !record.is_empty())
        .collect();
    if records.len() != SOURCE_RECORD_COUNT {
        return Err(CodegenError::new(
            CodegenErrorKind::WholeSourceAudit,
            format!(
                "expected {SOURCE_RECORD_COUNT} records, got {}",
                records.len()
            ),
        ));
    }

    let mut roots: BTreeMap<String, RootAudit> = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for (index, raw_record) in records.iter().enumerate() {
        let record = raw_record.strip_suffix(b"\r").ok_or_else(|| {
            CodegenError::new(
                CodegenErrorKind::SourceMetadata,
                format!("record {} lacks CR terminator", index + 1),
            )
        })?;
        let text = std::str::from_utf8(record).map_err(|error| {
            CodegenError::new(
                CodegenErrorKind::WholeSourceAudit,
                format!("record {} is not UTF-8: {error}", index + 1),
            )
        })?;
        let key = text.split_once(';').map(|(key, _)| key).ok_or_else(|| {
            CodegenError::new(
                CodegenErrorKind::WholeSourceAudit,
                format!("record {} has no semicolon", index + 1),
            )
        })?;
        let (root, place) = parse_place_key(key)?;
        if !keys.insert(key.to_owned()) {
            return Err(CodegenError::new(
                CodegenErrorKind::Duplicate,
                format!("duplicate source place key: {key}"),
            ));
        }
        let audit = roots.entry(root).or_insert_with(|| RootAudit {
            places: BTreeSet::new(),
            lines: Vec::new(),
        });
        audit.places.insert(place);
        audit.lines.push(index + 1);
    }
    let roots_with_x1 = roots
        .values()
        .filter(|audit| audit.places.contains(&1))
        .count();
    let gaps: Vec<String> = roots
        .iter()
        .flat_map(|(root, audit)| {
            let maximum = audit.places.last().copied().unwrap_or(0);
            (1..=maximum)
                .filter(|place| !audit.places.contains(place))
                .map(|place| format!("{root}{place}"))
                .collect::<Vec<_>>()
        })
        .collect();
    if roots.len() != SOURCE_ROOT_COUNT
        || roots_with_x1 != SOURCE_ROOTS_WITH_X1
        || gaps != ["moi1", "molki3"]
    {
        return Err(CodegenError::new(
            CodegenErrorKind::WholeSourceAudit,
            format!(
                "whole source audit drift: roots={}, x1={}, gaps={gaps:?}",
                roots.len(),
                roots_with_x1
            ),
        ));
    }
    audit_curated_metadata(&roots, &metadata)?;
    audit_mandatory_arities(&roots, &metadata)?;
    if sha256_hex(bytes) != SOURCE_SHA256 {
        return Err(CodegenError::new(
            CodegenErrorKind::SourceMetadata,
            "vendored source SHA-256 differs",
        ));
    }
    Ok(SourceAudit { roots, metadata })
}

#[requires(true)]
#[ensures(true)]
fn audit_curated_metadata(
    roots: &BTreeMap<String, RootAudit>,
    metadata: &SourceMetadata,
) -> Result<(), CodegenError> {
    if metadata.curated_root.len() != CURATED.len() {
        return Err(CodegenError::new(
            CodegenErrorKind::SourceMetadata,
            "curated-root sidecar count differs",
        ));
    }
    for expected in CURATED {
        let declared = metadata
            .curated_root
            .iter()
            .find(|root| root.root == expected.root)
            .ok_or_else(|| {
                CodegenError::new(
                    CodegenErrorKind::SourceMetadata,
                    format!("missing curated root {}", expected.root),
                )
            })?;
        let exact_declaration = declared.line_start == expected.line_start
            && declared.line_end == expected.line_end
            && declared.arity == expected.arity
            && declared.status == expected.status
            && declared.definition_id == expected.definition_id
            && declared.lexical_fingerprint == expected.fingerprint;
        if !exact_declaration {
            return Err(CodegenError::new(
                CodegenErrorKind::SourceMetadata,
                format!("curated sidecar drift for {}", expected.root),
            ));
        }
        let audit = roots.get(expected.root).ok_or_else(|| {
            CodegenError::new(
                CodegenErrorKind::KeyRange,
                format!("curated root absent from source: {}", expected.root),
            )
        })?;
        let expected_places: BTreeSet<usize> = (1..=expected.arity).collect();
        if audit.places != expected_places
            || audit.lines.first() != Some(&expected.line_start)
            || audit.lines.last() != Some(&expected.line_end)
        {
            return Err(CodegenError::new(
                CodegenErrorKind::KeyRange,
                format!("curated key range drift for {}", expected.root),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn audit_mandatory_arities(
    roots: &BTreeMap<String, RootAudit>,
    metadata: &SourceMetadata,
) -> Result<(), CodegenError> {
    let expected: BTreeMap<String, usize> = MANDATORY_ARITIES
        .iter()
        .map(|(root, arity)| ((*root).to_owned(), *arity))
        .collect();
    if metadata.mandatory_arities != expected {
        return Err(CodegenError::new(
            CodegenErrorKind::SourceMetadata,
            "mandatory arity sidecar differs",
        ));
    }
    for (root, arity) in MANDATORY_ARITIES {
        let expected_places: BTreeSet<usize> = (1..=*arity).collect();
        if roots.get(*root).map(|audit| &audit.places) != Some(&expected_places) {
            return Err(CodegenError::new(
                CodegenErrorKind::KeyRange,
                format!("mandatory root {root} is not contiguous through x{arity}"),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn audit_witnesses(bytes: &[u8]) -> Result<(), CodegenError> {
    if bytes != WITNESSES.as_bytes()
        || sha256_hex(bytes) != WITNESS_SHA256
        || bytes.iter().filter(|byte| **byte == b'\n').count() != 18
    {
        return Err(CodegenError::new(
            CodegenErrorKind::Coverage,
            "must-compact witness registry differs from the frozen 18 inputs",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn audit_dictionary(
    bytes: &[u8],
    metadata_text: &str,
    source: &SourceAudit,
) -> Result<(), CodegenError> {
    let metadata: DictionaryMetadata = toml::from_str(metadata_text).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("parse dictionary metadata: {error}"),
        )
    })?;
    if metadata.sha256 != DICTIONARY_SHA256
        || metadata.lensisku_created_at != DICTIONARY_CREATED_AT
        || metadata.entry_count != DICTIONARY_ENTRY_COUNT
        || source.metadata.lensisku_sha256 != metadata.sha256
    {
        return Err(CodegenError::new(
            CodegenErrorKind::LexicalIdentity,
            "Lensisku snapshot identity or metadata differs",
        ));
    }
    let entries: Vec<Value> = serde_json::from_slice(bytes).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("parse Lensisku JSON: {error}"),
        )
    })?;
    if entries.len() != DICTIONARY_ENTRY_COUNT {
        return Err(CodegenError::new(
            CodegenErrorKind::LexicalIdentity,
            format!("Lensisku entry count is {}", entries.len()),
        ));
    }
    let expected_key_counts: BTreeMap<&str, usize> = [
        ("definition", 17_536),
        ("definition_id", 17_536),
        ("etymology", 1_508),
        ("gloss_keywords", 15_132),
        ("jargon", 7_887),
        ("notes", 17_344),
        ("place_keywords", 2_510),
        ("rafsi", 1_208),
        ("score", 17_536),
        ("selmaho", 2_338),
        ("user", 17_536),
        ("word", 17_536),
        ("word_type", 17_536),
    ]
    .into_iter()
    .collect();
    let mut key_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in &entries {
        let object = entry.as_object().ok_or_else(|| {
            CodegenError::new(
                CodegenErrorKind::LexicalIdentity,
                "dictionary row is not an object",
            )
        })?;
        for key in object.keys() {
            *key_counts.entry(key.as_str()).or_default() += 1;
        }
    }
    if key_counts != expected_key_counts {
        return Err(CodegenError::new(
            CodegenErrorKind::LexicalIdentity,
            "Lensisku key union or presence counts differ",
        ));
    }
    let gismu: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry.get("word_type").and_then(Value::as_str) == Some("gismu"))
        .collect();
    if gismu.len() != DICTIONARY_GISMU_COUNT
        || gismu.iter().any(|entry| {
            entry
                .get("word")
                .and_then(Value::as_str)
                .is_none_or(|root| {
                    !source
                        .roots
                        .get(root)
                        .is_some_and(|audit| audit.places.contains(&1))
                })
        })
    {
        return Err(CodegenError::new(
            CodegenErrorKind::LexicalIdentity,
            "Lensisku gismu/x1 coverage differs",
        ));
    }
    for expected in CURATED {
        let matches: Vec<&Value> = gismu
            .iter()
            .copied()
            .filter(|entry| entry.get("word").and_then(Value::as_str) == Some(expected.root))
            .collect();
        if matches.len() != 1 {
            return Err(CodegenError::new(
                CodegenErrorKind::LexicalIdentity,
                format!(
                    "{} has {} exact gismu identities",
                    expected.root,
                    matches.len()
                ),
            ));
        }
        let entry = matches[0];
        if entry.get("definition_id").and_then(Value::as_u64) != Some(expected.definition_id) {
            return Err(CodegenError::new(
                CodegenErrorKind::LexicalIdentity,
                format!("definition ID drift for {}", expected.root),
            ));
        }
        if lexical_fingerprint(entry)? != expected.fingerprint {
            return Err(CodegenError::new(
                CodegenErrorKind::Fingerprint,
                format!("lexical fingerprint drift for {}", expected.root),
            ));
        }
    }
    if sha256_hex(bytes) != DICTIONARY_SHA256 {
        return Err(CodegenError::new(
            CodegenErrorKind::LexicalIdentity,
            "Lensisku snapshot SHA-256 differs",
        ));
    }
    Ok(())
}

#[requires(entry.is_object())]
#[ensures(ret.as_ref().is_ok_and(|hash| hash.len() == 64) || ret.is_err())]
fn lexical_fingerprint(entry: &Value) -> Result<String, CodegenError> {
    let object = entry.as_object().expect("precondition requires object");
    let mut canonical = BTreeMap::new();
    for key in ["word", "word_type", "definition_id", "definition", "notes"] {
        canonical.insert(
            key,
            object.get(key).cloned().ok_or_else(|| {
                CodegenError::new(
                    CodegenErrorKind::LexicalIdentity,
                    format!("fingerprint field absent: {key}"),
                )
            })?,
        );
    }
    let mut bytes = serde_json::to_vec(&canonical).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("serialize canonical fingerprint: {error}"),
        )
    })?;
    bytes.push(b'\n');
    Ok(sha256_hex(&bytes))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == 7) || ret.is_err())]
fn audit_policies(text: &str, source: &SourceAudit) -> Result<Vec<GeneratedRow>, CodegenError> {
    let policy_file: PolicyFile = toml::from_str(text).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("parse policy source: {error}"),
        )
    })?;
    if policy_file.relation_place.len() != EXPECTED_POLICY_KEYS.len() {
        return Err(CodegenError::new(
            CodegenErrorKind::Coverage,
            "policy source must contain exactly seven outer rows",
        ));
    }
    let source_order: Vec<(String, usize)> = policy_file
        .relation_place
        .iter()
        .map(|row| (row.relation.clone(), row.original_place))
        .collect();
    let mut sorted_order = source_order.clone();
    sorted_order.sort();
    if source_order != sorted_order {
        return Err(CodegenError::new(
            CodegenErrorKind::NonDeterministicOrder,
            "policy rows must be in canonical relation/place order",
        ));
    }
    let mut seen_outer = BTreeSet::new();
    let mut rows = Vec::new();
    for row in policy_file.relation_place {
        if !seen_outer.insert((row.relation.clone(), row.original_place)) {
            return Err(CodegenError::new(
                CodegenErrorKind::Duplicate,
                format!(
                    "duplicate outer policy row: {} x{}",
                    row.relation, row.original_place
                ),
            ));
        }
        if !is_canonical_relation(&row.relation) {
            return Err(CodegenError::new(
                CodegenErrorKind::LexicalIdentity,
                format!("noncanonical relation identity: {}", row.relation),
            ));
        }
        let expected = EXPECTED_POLICY_KEYS
            .iter()
            .find(|(relation, place, _, _)| {
                *relation == row.relation && *place == row.original_place
            })
            .ok_or_else(|| {
                CodegenError::new(
                    CodegenErrorKind::Coverage,
                    format!(
                        "orphan policy row: {} x{}",
                        row.relation, row.original_place
                    ),
                )
            })?;
        let curated = CURATED
            .iter()
            .find(|curated| curated.root == row.relation)
            .expect("expected policy roots are curated");
        let source_root = source.roots.get(&row.relation).ok_or_else(|| {
            CodegenError::new(
                CodegenErrorKind::KeyRange,
                format!("policy relation absent from source: {}", row.relation),
            )
        })?;
        let expected_places: BTreeSet<usize> = (1..=row.attested_arity).collect();
        if row.original_place == 0
            || row.original_place > row.attested_arity
            || row.attested_arity != curated.arity
            || source_root.places != expected_places
        {
            return Err(CodegenError::new(
                CodegenErrorKind::KeyRange,
                format!(
                    "policy key/arity drift for {} x{}",
                    row.relation, row.original_place
                ),
            ));
        }
        if row.word_type != "gismu"
            || row.definition_id != curated.definition_id
            || row.lexical_fingerprint != format!("sha256:{}", curated.fingerprint)
        {
            return Err(CodegenError::new(
                CodegenErrorKind::LexicalIdentity,
                format!("policy lexical identity drift for {}", row.relation),
            ));
        }
        if row.family_policy.len() != 1 {
            return Err(CodegenError::new(
                CodegenErrorKind::Duplicate,
                format!(
                    "{} x{} must have one unique family child",
                    row.relation, row.original_place
                ),
            ));
        }
        if row.place_key_source.trim().is_empty() || !nonempty_evidence(&row.identity_evidence) {
            return Err(CodegenError::new(
                CodegenErrorKind::Evidence,
                format!(
                    "identity evidence is empty for {} x{}",
                    row.relation, row.original_place
                ),
            ));
        }
        let family = &row.family_policy[0];
        let family_variant = match family.dynamic_family.as_str() {
            "ref-comp-referents-entity" => "RefCompReferentsEntity",
            "ref-comp-referents-eventuality" => "RefCompReferentsEventuality",
            other => {
                return Err(CodegenError::new(
                    CodegenErrorKind::ClosedEnum,
                    format!("unknown dynamic family: {other}"),
                ));
            }
        };
        if family.dynamic_family != expected.2 {
            return Err(CodegenError::new(
                CodegenErrorKind::Coverage,
                format!(
                    "unsupported family for {} x{}",
                    row.relation, row.original_place
                ),
            ));
        }
        let policy_variant = match family.policy.as_str() {
            "Extensional" => "Extensional",
            "Intensional" => "Intensional",
            "Opaque" => "Opaque",
            other => {
                return Err(CodegenError::new(
                    CodegenErrorKind::ClosedEnum,
                    format!("unknown scope policy: {other}"),
                ));
            }
        };
        if !nonempty_evidence(&family.family_evidence)
            || !nonempty_evidence(&family.policy_evidence)
            || !nonempty_evidence(&family.negative_evidence)
            || !nonempty_evidence(&family.coverage)
            || family.rationale.trim().is_empty()
        {
            return Err(CodegenError::new(
                CodegenErrorKind::Evidence,
                format!(
                    "family/policy evidence is empty for {} x{}",
                    row.relation, row.original_place
                ),
            ));
        }
        let expected_coverage: BTreeSet<String> = expected
            .3
            .iter()
            .map(|witness| format!("must:{witness}"))
            .collect();
        let actual_coverage: BTreeSet<String> = family.coverage.iter().cloned().collect();
        if actual_coverage != expected_coverage || actual_coverage.len() != family.coverage.len() {
            return Err(CodegenError::new(
                CodegenErrorKind::Coverage,
                format!(
                    "coverage tags drift for {} x{}",
                    row.relation, row.original_place
                ),
            ));
        }
        rows.push(new!(GeneratedRow {
            relation: row.relation,
            original_place: row.original_place,
            attested_arity: row.attested_arity,
            family_variant,
            policy_variant,
        }));
    }
    let actual_keys: BTreeSet<(String, usize, String)> = rows
        .iter()
        .map(|row| {
            let family = match row.family_variant {
                "RefCompReferentsEntity" => "ref-comp-referents-entity",
                "RefCompReferentsEventuality" => "ref-comp-referents-eventuality",
                _ => unreachable!("closed generator family"),
            };
            (row.relation.clone(), row.original_place, family.to_owned())
        })
        .collect();
    let expected_keys: BTreeSet<(String, usize, String)> = EXPECTED_POLICY_KEYS
        .iter()
        .map(|(relation, place, family, _)| ((*relation).to_owned(), *place, (*family).to_owned()))
        .collect();
    if actual_keys != expected_keys {
        return Err(CodegenError::new(
            CodegenErrorKind::Coverage,
            "missing admitted edge or orphan policy row",
        ));
    }
    Ok(rows)
}

#[requires(true)]
#[ensures(ret == (!values.is_empty() && values.iter().all(|value| !value.trim().is_empty())))]
fn nonempty_evidence(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| !value.trim().is_empty())
}

#[requires(!rows.is_empty())]
#[ensures(!ret.is_empty())]
fn render(rows: &[GeneratedRow]) -> Vec<u8> {
    let mut output =
        String::from("// @generated by codegen/lexical_scope_policy.rs; do not edit.\n\n");
    output.push_str("const GENERATED_LEXICAL_POLICY_ROWS: &[GeneratedLexicalPolicyRow] = &[\n");
    for row in rows {
        writeln!(
            output,
            "    GeneratedLexicalPolicyRow {{ relation: {:?}, original_place: {}, attested_arity: {}, family: DynamicValueFamily::{}, policy: ScopePolicy::{} }},",
            row.relation,
            row.original_place,
            row.attested_arity,
            row.family_variant,
            row.policy_variant,
        )
        .expect("writing to String is infallible");
    }
    output.push_str("];\n");
    output.into_bytes()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| !bytes.is_empty()) || ret.is_err())]
pub fn generate(inputs: GeneratorInputs<'_>) -> Result<Vec<u8>, CodegenError> {
    let source = audit_source(inputs.source, inputs.source_metadata)?;
    audit_witnesses(inputs.witnesses)?;
    audit_dictionary(inputs.dictionary, inputs.dictionary_metadata, &source)?;
    let rows = audit_policies(inputs.policies, &source)?;
    let first = render(&rows);
    let second = render(&rows);
    if first != second {
        return Err(CodegenError::new(
            CodegenErrorKind::NonDeterministicOrder,
            "identical audited inputs generated different bytes",
        ));
    }
    Ok(first)
}

#[requires(true)]
#[ensures(ret.is_ok() -> paths.output.is_file())]
pub fn generate_from_paths(paths: &GeneratorPaths) -> Result<(), CodegenError> {
    let read = |path: &Path| {
        fs::read(path).map_err(|error| {
            CodegenError::new(
                CodegenErrorKind::Io,
                format!("read {}: {error}", path.display()),
            )
        })
    };
    let source = read(&paths.source)?;
    let source_metadata = read(&paths.source_metadata)?;
    let policies = read(&paths.policies)?;
    let witnesses = read(&paths.witnesses)?;
    let dictionary = read(&paths.dictionary)?;
    let dictionary_metadata = read(&paths.dictionary_metadata)?;
    let source_metadata = std::str::from_utf8(&source_metadata).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("source metadata is not UTF-8: {error}"),
        )
    })?;
    let policies = std::str::from_utf8(&policies).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("policy source is not UTF-8: {error}"),
        )
    })?;
    let dictionary_metadata = std::str::from_utf8(&dictionary_metadata).map_err(|error| {
        CodegenError::new(
            CodegenErrorKind::Parse,
            format!("dictionary metadata is not UTF-8: {error}"),
        )
    })?;
    let generated = generate(GeneratorInputs {
        source: &source,
        source_metadata,
        policies,
        witnesses: &witnesses,
        dictionary: &dictionary,
        dictionary_metadata,
    })?;
    if fs::read(&paths.output).ok().as_deref() != Some(generated.as_slice()) {
        fs::write(&paths.output, generated).map_err(|error| {
            CodegenError::new(
                CodegenErrorKind::Io,
                format!("write {}: {error}", paths.output.display()),
            )
        })?;
    }
    Ok(())
}

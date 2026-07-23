use std::collections::BTreeMap;

use anyhow::{Result, bail};
use bityzba::{ensures, invariant, new, requires};

use super::classification::{ClassificationCatalog, MigrationRule, RecordFamily};
use super::model::MigrationLedgerRecord;

#[invariant(!id.is_empty() && !path.is_empty())]
#[derive(Debug, Clone)]
pub(crate) struct ClassifiableRecord {
    pub(crate) id: String,
    pub(crate) family: RecordFamily,
    pub(crate) path: String,
    pub(crate) owner_type: Option<String>,
}

#[requires(!records.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|(ledger, _)| ledger.len() == records.len()))]
pub(crate) fn build_migration_ledger(
    records: &[ClassifiableRecord],
    catalog: &ClassificationCatalog,
) -> Result<(Vec<MigrationLedgerRecord>, BTreeMap<String, usize>)> {
    let mut rule_hits = catalog
        .migration_rules
        .iter()
        .map(|rule| (rule.id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for rule in &catalog.migration_rules {
        validate_decision_ownership(rule)?;
    }
    let mut ledger = Vec::with_capacity(records.len());
    let mut classification_errors = Vec::new();
    for record in records {
        let matches = catalog
            .migration_rules
            .iter()
            .filter(|rule| rule_matches(rule, record))
            .collect::<Vec<_>>();
        let [rule] = matches.as_slice() else {
            let matching_ids = matches
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>();
            classification_errors.push(format!(
                "record `{}` ({}, path `{}`, owner type {:?}) matched {} migration rules: {:?}",
                record.id,
                record.family.as_str(),
                record.path,
                record.owner_type,
                matches.len(),
                matching_ids
            ));
            continue;
        };
        *rule_hits
            .get_mut(&rule.id)
            .expect("every migration rule initialized a hit counter") += 1;
        ledger.push(new!(MigrationLedgerRecord {
            id: format!("migration:{}", record.id),
            record_id: record.id.clone(),
            record_kind: record.family.as_str().to_owned(),
            disposition: rule.disposition,
            primary_owner: rule.primary_owner,
            additional_owners: rule.additional_owners.clone(),
            decision_ids: rule.decision_ids.clone(),
            rationale: rule.rationale.clone(),
        }));
    }
    if !classification_errors.is_empty() {
        bail!(
            "migration classification failed for {} records:\n{}",
            classification_errors.len(),
            classification_errors.join("\n")
        );
    }
    let unused = rule_hits
        .iter()
        .filter_map(|(id, hits)| (*hits == 0).then_some(id.as_str()))
        .collect::<Vec<_>>();
    if !unused.is_empty() {
        bail!("migration classification catalog has unused rules: {unused:?}");
    }
    ledger.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((ledger, rule_hits))
}

#[requires(true)]
#[ensures(true)]
fn rule_matches(rule: &MigrationRule, record: &ClassifiableRecord) -> bool {
    rule.paths.contains(&record.path)
        && rule.record_families.contains(&record.family)
        && (rule.record_ids.is_empty() || rule.record_ids.contains(&record.id))
        && !rule.exclude_record_ids.contains(&record.id)
        && (rule.owner_types.is_empty()
            || record
                .owner_type
                .as_ref()
                .is_some_and(|owner| rule.owner_types.contains(owner)))
        && record
            .owner_type
            .as_ref()
            .is_none_or(|owner| !rule.exclude_owner_types.contains(owner))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_decision_ownership(rule: &MigrationRule) -> Result<()> {
    for decision in &rule.decision_ids {
        let expected_owner = match decision.as_str() {
            "D1" => 570,
            "D2" => 571,
            "D3" => 572,
            "D4" => 573,
            "D5" => 574,
            "D6" | "D7" => 575,
            "D8" | "D9" | "D10" => 576,
            "D11" => 577,
            "D12" => 578,
            _ => bail!(
                "migration rule `{}` contains unknown decision `{decision}`",
                rule.id
            ),
        };
        if rule.primary_owner != expected_owner
            && !rule.additional_owners.contains(&expected_owner)
        {
            bail!(
                "migration rule `{}` does not assign {decision} to expected owner #{expected_owner}",
                rule.id
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::new;

    use super::super::classification::{ExecutableConsumerRule, PythonFileRule};
    use super::super::model::{
        ConsumerLanguage, LifecycleDisposition, MigrationDisposition, PythonCoverageClass,
    };

    #[requires(!rules.is_empty())]
    #[ensures(!ret.migration_rules.is_empty())]
    fn catalog(rules: Vec<MigrationRule>) -> ClassificationCatalog {
        new!(ClassificationCatalog {
            version: 1,
            python_files: vec![new!(PythonFileRule {
                path: "sample.py".to_owned(),
                class: PythonCoverageClass::BindingTest,
                reason: "test classification".to_owned(),
            })],
            executable_consumers: vec![new!(ExecutableConsumerRule {
                path: "sample.rs".to_owned(),
                language: ConsumerLanguage::Rust,
                consumer_kind: "test".to_owned(),
                cli_formats: Vec::new(),
                cli_default: None,
                error_surfaces: Vec::new(),
                fixture_reads: Vec::new(),
                fixture_writes: Vec::new(),
                hard_coded_tokens: Vec::new(),
                base_revision_assumptions: Vec::new(),
                current_revision_assumptions: Vec::new(),
                binary_assumptions: Vec::new(),
                exact_invocation: "test".to_owned(),
                allowlist_count_hash_pins: Vec::new(),
                downstream_migration_owners: vec![569],
                lifecycle_disposition: LifecycleDisposition::RetainAndRun,
                replacement_gate: None,
                preserved_witnesses: BTreeMap::new(),
            })],
            migration_rules: rules,
        })
    }

    #[requires(!id.is_empty() && !path.is_empty())]
    #[ensures(ret.id == id && ret.path == path)]
    fn record(id: &str, path: &str) -> ClassifiableRecord {
        new!(ClassifiableRecord {
            id: id.to_owned(),
            family: RecordFamily::Type,
            path: path.to_owned(),
            owner_type: Some("Sample".to_owned()),
        })
    }

    #[requires(!id.is_empty() && !path.is_empty())]
    #[ensures(ret.id == id)]
    fn rule(id: &str, path: &str, owner: u16, decisions: Vec<String>) -> MigrationRule {
        new!(MigrationRule {
            id: id.to_owned(),
            paths: vec![path.to_owned()],
            record_families: vec![RecordFamily::Type],
            record_ids: Vec::new(),
            exclude_record_ids: Vec::new(),
            owner_types: vec!["Sample".to_owned()],
            exclude_owner_types: Vec::new(),
            disposition: MigrationDisposition::Replace,
            primary_owner: owner,
            additional_owners: Vec::new(),
            decision_ids: decisions,
            rationale: "test rule".to_owned(),
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ledger_requires_exactly_one_rule_per_record() {
        let records = vec![record("record:one", "one.rs")];
        let valid_catalog = catalog(vec![rule("valid", "one.rs", 570, vec!["D1".to_owned()])]);
        let (ledger, hits) = build_migration_ledger(&records, &valid_catalog)
            .expect("one exact classification succeeds");
        assert_eq!(ledger.len(), 1);
        assert_eq!(hits["valid"], 1);

        let ambiguous_catalog = catalog(vec![
            rule("first", "one.rs", 570, vec!["D1".to_owned()]),
            rule("second", "one.rs", 570, vec!["D1".to_owned()]),
        ]);
        let ambiguous = build_migration_ledger(&records, &ambiguous_catalog)
            .expect_err("overlapping rules must fail");
        assert!(ambiguous.to_string().contains("matched 2 migration rules"));

        let unmatched_catalog = catalog(vec![rule("unused", "two.rs", 570, vec!["D1".to_owned()])]);
        let unmatched = build_migration_ledger(&records, &unmatched_catalog)
            .expect_err("unclassified records must fail");
        assert!(unmatched.to_string().contains("matched 0 migration rules"));

        let unused_catalog = catalog(vec![
            rule("used", "one.rs", 570, vec!["D1".to_owned()]),
            rule("unused", "two.rs", 570, vec!["D1".to_owned()]),
        ]);
        let unused = build_migration_ledger(&records, &unused_catalog)
            .expect_err("unused classification rules must fail");
        assert!(unused.to_string().contains("unused rules"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decision_ids_are_bound_to_their_exact_owning_issue() {
        let records = vec![record("record:one", "one.rs")];
        let invalid_catalog = catalog(vec![rule(
            "wrong-owner",
            "one.rs",
            571,
            vec!["D1".to_owned()],
        )]);
        let error = build_migration_ledger(&records, &invalid_catalog)
            .expect_err("D1 cannot be assigned to the D2 issue");
        assert!(error.to_string().contains("expected owner #570"));
    }
}

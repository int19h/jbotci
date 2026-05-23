use bityzba::{contract_trait, ensures, invariant, requires};
use rayon::prelude::*;

use super::{Facet, LoadedTestCase};

#[contract_trait]
pub trait FixtureBackend {
    #[requires(true)]
    #[ensures(true)]
    fn run(&self, fixture: &LoadedTestCase, facet: Facet) -> FacetResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct FacetResult {
    pub status: FacetStatus,
    pub message: Option<String>,
}

impl FacetResult {
    #[ensures(ret.is_valid())]
    #[requires(true)]
    pub fn passed() -> Self {
        Self {
            status: FacetStatus::Passed,
            message: None,
        }
    }

    #[ensures(ret.is_valid())]
    #[requires(true)]
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            status: FacetStatus::Failed,
            message: Some(message.into()),
        }
    }

    #[ensures(ret.is_valid())]
    #[requires(true)]
    pub fn skipped(message: impl Into<String>) -> Self {
        Self {
            status: FacetStatus::Skipped,
            message: Some(message.into()),
        }
    }

    #[ensures(ret.is_valid())]
    #[requires(true)]
    pub fn xfailed(message: impl Into<String>) -> Self {
        Self {
            status: FacetStatus::Xfailed,
            message: Some(message.into()),
        }
    }

    #[ensures(ret -> (self.status == FacetStatus::Passed) == self.message.is_none())]
    #[ensures(ret -> self.message.as_ref().is_none_or(|message| !message.is_empty()))]
    #[requires(true)]
    pub fn is_valid(&self) -> bool {
        match self.status {
            FacetStatus::Passed => self.message.is_none(),
            FacetStatus::Failed | FacetStatus::Skipped | FacetStatus::Xfailed => self
                .message
                .as_ref()
                .is_some_and(|message| !message.is_empty()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetStatus {
    Passed,
    Failed,
    Skipped,
    Xfailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[invariant(true)]
pub struct RunSummary {
    pub selected_fixtures: usize,
    pub selected_facets: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub xfailed: usize,
}

impl RunSummary {
    #[ensures(ret == self.passed + self.failed + self.skipped + self.xfailed)]
    #[requires(true)]
    pub fn total_results(&self) -> usize {
        self.passed + self.failed + self.skipped + self.xfailed
    }

    #[requires(result.is_valid())]
    #[ensures(true)]
    pub fn record_result(&mut self, result: &FacetResult) {
        match result.status {
            FacetStatus::Passed => self.passed += 1,
            FacetStatus::Failed => self.failed += 1,
            FacetStatus::Skipped => self.skipped += 1,
            FacetStatus::Xfailed => self.xfailed += 1,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn merge(&mut self, other: Self) {
        self.selected_fixtures += other.selected_fixtures;
        self.passed += other.passed;
        self.failed += other.failed;
        self.skipped += other.skipped;
        self.xfailed += other.xfailed;
    }
}

#[ensures(ret.selected_fixtures == fixtures.len())]
#[ensures(ret.selected_facets == facets.len())]
#[ensures(ret.total_results() == fixtures.len() * facets.len())]
#[requires(true)]
pub fn run_fixture_facets<B: FixtureBackend>(
    backend: &B,
    fixtures: &[&LoadedTestCase],
    facets: &[Facet],
) -> RunSummary {
    let mut summary = RunSummary {
        selected_fixtures: fixtures.len(),
        selected_facets: facets.len(),
        ..RunSummary::default()
    };
    for fixture in fixtures {
        for facet in facets {
            summary.record_result(&backend.run(fixture, *facet));
        }
    }
    summary
}

#[ensures(ret.selected_fixtures == fixtures.len())]
#[ensures(ret.selected_facets == facets.len())]
#[ensures(ret.total_results() == fixtures.len() * facets.len())]
#[requires(true)]
pub fn run_fixture_facets_parallel<B: FixtureBackend + Sync>(
    backend: &B,
    fixtures: &[&LoadedTestCase],
    facets: &[Facet],
) -> RunSummary {
    let mut summary = fixtures
        .par_iter()
        .map(|fixture| {
            let mut summary = RunSummary {
                selected_fixtures: 1,
                ..RunSummary::default()
            };
            for facet in facets {
                summary.record_result(&backend.run(fixture, *facet));
            }
            summary
        })
        .reduce(RunSummary::default, |mut left, right| {
            left.merge(right);
            left
        });
    summary.selected_facets = facets.len();
    summary
}

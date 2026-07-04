use super::super::*;

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_setup<WOut: Write>(
    input: SetupInput,
    stdout: &mut WOut,
    progress_policy: CliProgressPolicy,
) -> Result<CliStatus> {
    if !input.embedding {
        bail!("Choose at least one setup task, e.g. `jbotci setup --embedding`.");
    }
    let mut reporter = CliSetupProgressReporter::new(progress_policy.embedding_setup);
    let mut progress = |progress: SetupProgress| {
        reporter.update(&progress);
    };
    let report = match setup_embeddings_with_progress(
        &SetupOptions {
            model_key: input.model,
            force: input.force,
            use_precomputed: input.use_precomputed.into(),
            skip_validation: input.skip_validation,
            index_dir: input.index_dir,
            model_dir: input.model_dir,
            ..SetupOptions::default()
        },
        &mut progress,
    ) {
        Ok(report) => {
            reporter.finish();
            report
        }
        Err(error) => {
            reporter.fail();
            return Err(anyhow!(error.to_string()));
        }
    };
    writeln!(
        stdout,
        "Embedding setup complete.\nmodel: {}\nindex: {}\npack: {}\nsource: {}\ndictionary rows: {}\nCLL rows: {}",
        report
            .model_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not checked".to_owned()),
        report.index_root.display(),
        report.pack_id,
        report.index_source.as_str(),
        report.dictionary_rows,
        report.cll_rows
    )?;
    Ok(CliStatus::Success)
}

#[invariant(true)]
#[derive(Debug)]
struct CliSetupProgressReporter {
    job: Option<std::sync::Arc<clx::progress::ProgressJob>>,
    determinate: bool,
}

impl CliSetupProgressReporter {
    #[requires(true)]
    #[ensures(enabled -> ret.job.is_some() || clx::progress::is_disabled())]
    fn new(enabled: bool) -> Self {
        if !enabled || clx::progress::is_disabled() {
            return Self {
                job: None,
                determinate: false,
            };
        }
        let job = ProgressJobBuilder::new()
            .body("{{ spinner() }} {{ message }} {{ detail | flex }}")
            .prop("message", "Embedding setup")
            .prop("detail", "")
            .start();
        Self {
            job: Some(job),
            determinate: false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn update(&mut self, progress: &SetupProgress) {
        let Some(job) = &self.job else {
            return;
        };
        let detail = cli_setup_progress_detail(progress);
        if let (Some(loaded), Some(total)) = (progress.loaded, progress.total) {
            if !self.determinate {
                job.set_body("{{ spinner() }} {{ message }} {{ detail | flex }} {{ progress_bar(flex=true) }}");
                self.determinate = true;
            }
            job.progress_total(usize::try_from(total).unwrap_or(usize::MAX));
            job.progress_current(usize::try_from(loaded).unwrap_or(usize::MAX));
        } else if self.determinate {
            job.set_body("{{ spinner() }} {{ message }} {{ detail | flex }}");
            self.determinate = false;
        }
        job.message(&progress.label);
        job.prop("detail", &detail);
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(&mut self) {
        if let Some(job) = &self.job {
            job.set_status(ProgressStatus::Done);
            clx::progress::stop_clear();
        }
        self.job = None;
    }

    #[requires(true)]
    #[ensures(true)]
    fn fail(&mut self) {
        if let Some(job) = &self.job {
            job.set_status(ProgressStatus::Failed);
            clx::progress::stop_clear();
        }
        self.job = None;
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cli_setup_progress_detail(progress: &SetupProgress) -> String {
    if let (Some(loaded), Some(total)) = (progress.loaded, progress.total) {
        return match progress.kind.as_str() {
            "download" | "validate" => format!("{} / {}", human_bytes(loaded), human_bytes(total)),
            _ => format!("{loaded}/{total} rows"),
        };
    }
    if progress.detail.is_empty() {
        return progress.label.clone();
    }
    progress.detail.clone()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

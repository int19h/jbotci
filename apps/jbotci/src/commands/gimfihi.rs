use super::super::*;

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_gimfihi<WOut: Write>(
    input: GimfihiInput,
    stdout: &mut WOut,
) -> Result<CliStatus> {
    let request = gimfihi_request_from_input(&input)?;
    let output = compose_gismu(jbotci_dictionary_data::english(), &request)
        .map_err(|error| anyhow!(error.to_string()))?;
    match input.format {
        GimfihiCliFormat::Table => writeln!(stdout, "{}", render_gimfihi_table(&output))?,
        GimfihiCliFormat::Json => {
            writeln!(
                stdout,
                "{}",
                serde_json::to_string_pretty(&output)
                    .context("failed to serialize gimfihi output")?
            )?;
        }
    }
    Ok(CliStatus::Success)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn gimfihi_request_from_input(input: &GimfihiInput) -> Result<GimfihiRequest> {
    let count = input.count.unwrap_or(GIMFIHI_DEFAULT_COUNT);
    if count == 0 {
        bail!("`--count` must be greater than 0");
    }
    if count > GIMFIHI_MAX_COUNT {
        bail!("`--count` must be at most {GIMFIHI_MAX_COUNT}");
    }
    let preset = input
        .preset
        .as_deref()
        .map(parse_preset)
        .transpose()
        .map_err(|error| anyhow!(error.to_string()))?;
    let sources = input.sources.clone();
    let shapes = if input.shapes.is_empty() {
        default_shapes()
    } else {
        input
            .shapes
            .iter()
            .map(|shape| parse_shape(shape).map_err(|error| anyhow!(error.to_string())))
            .collect::<Result<Vec<_>>>()?
    };
    let mut saliences = AlineSaliences::default();
    for override_value in &input.saliences {
        saliences = saliences
            .with_feature(override_value.feature, override_value.value)
            .map_err(|error| anyhow!(error.to_string()))?;
    }
    let phonetic_parameters = AlineParameters::try_new(
        saliences,
        input.c_sub,
        input.c_exp,
        input.c_skip,
        input.c_vwl,
        input.c_flank,
        input.normalizer.into(),
    )
    .map_err(|error| anyhow!(error.to_string()))?;
    if input.scorer == GimfihiCliScorer::Classic
        && phonetic_parameters != AlineParameters::default()
    {
        bail!("non-default phonetic parameters require `--scorer phonetic`");
    }
    Ok(GimfihiRequest {
        scorer: input.scorer.into(),
        phonetic_parameters,
        preset,
        sources,
        shapes,
        all_letters: input.all_letters,
        check_collisions: input.check_collisions.into(),
        show_collisions: input.show_collisions,
        require_free_short_rafsi: input.require_free_short_rafsi,
        count,
        highlight: input.highlight.clone(),
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_gimfihi_table(output: &GimfihiOutput) -> String {
    if output.candidates.is_empty() {
        return "No gismu candidates matched the selected filters.".to_owned();
    }
    let mut lines = Vec::new();
    if output.scorer != GimfihiScorer::Classic {
        lines.push(format!("scorer: {}", output.scorer.as_str()));
    }
    if let Some(parameters) = &output.phonetic_parameters {
        lines.push(format_nondefault_phonetic_parameters(parameters));
    }
    lines.push(format!(
        "winner: {}",
        output.winner.as_deref().unwrap_or("none")
    ));
    lines.push(format!(
        "candidates: {} shown of {} passing ({} valid)",
        output.candidates.len(),
        output.filtered_count,
        output.candidate_count
    ));
    lines.push("mark  gismu  score     rafsi".to_owned());
    for candidate in &output.candidates {
        lines.push(render_gimfihi_candidate_row(candidate));
    }
    lines.join("\n")
}

#[requires(parameters != &AlineParameters::default())]
#[ensures(ret.starts_with("parameters:"))]
fn format_nondefault_phonetic_parameters(parameters: &AlineParameters) -> String {
    let defaults = AlineParameters::default();
    let mut values = Vec::new();
    for (name, value, default) in [
        ("c-sub", parameters.c_sub, defaults.c_sub),
        ("c-exp", parameters.c_exp, defaults.c_exp),
        ("c-skip", parameters.c_skip, defaults.c_skip),
        ("c-vwl", parameters.c_vwl, defaults.c_vwl),
        ("c-flank", parameters.c_flank, defaults.c_flank),
    ] {
        if value != default {
            values.push(format!("{name}={}", format_gimfihi_score(value)));
        }
    }
    if parameters.normalizer != defaults.normalizer {
        values.push(format!(
            "normalizer={}",
            match parameters.normalizer {
                AlineNormalizer::SourceSide => "source-side",
                AlineNormalizer::CandidateSide => "candidate-side",
                AlineNormalizer::Symmetric => "symmetric",
            }
        ));
    }
    for feature in AlineFeature::all() {
        let value = parameters.saliences.value(*feature);
        if value != defaults.saliences.value(*feature) {
            values.push(format!(
                "{}={}",
                feature.as_str(),
                format_gimfihi_score(value)
            ));
        }
    }
    format!("parameters: {}", values.join(", "))
}

#[requires(!candidate.word.is_empty())]
#[ensures(!ret.is_empty())]
fn render_gimfihi_candidate_row(candidate: &GimfihiCandidate) -> String {
    let marker = if candidate.highlighted { "*" } else { " " };
    let collision = candidate
        .collision
        .as_ref()
        .map(|collision| format!("{} ", format_gimfihi_collision(collision)))
        .unwrap_or_default();
    format!(
        "{marker}     {:<5}  {:<8} {collision}{}",
        candidate.word,
        format_gimfihi_score(candidate.score),
        format_gimfihi_rafsi(candidate)
    )
}

/// Render a candidate's gismu-level collision with an existing word.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_gimfihi_collision(collision: &GismuCollision) -> String {
    match collision.kind {
        CollisionKind::Identical => {
            format!("[= existing {}]", collision.existing_word_type.as_str())
        }
        CollisionKind::FinalVowel => format!("[~ {}: final vowel]", collision.existing_word),
        CollisionKind::SimilarConsonant => {
            format!("[~ {}: similar consonant]", collision.existing_word)
        }
    }
}

#[requires(score.is_finite())]
#[ensures(!ret.is_empty())]
fn format_gimfihi_score(score: f64) -> String {
    trim_float(&format!("{score:.6}"))
}

#[requires(!candidate.word.is_empty())]
#[ensures(true)]
fn format_gimfihi_rafsi(candidate: &GimfihiCandidate) -> String {
    if candidate.rafsi().is_empty() {
        return String::new();
    }
    candidate
        .rafsi()
        .iter()
        .map(|rafsi| {
            let status = match rafsi.availability {
                RafsiAvailability::Free => "free".to_owned(),
                RafsiAvailability::OfficialTaken => format!(
                    "official-taken{}",
                    format_taken_rafsi_sources(&rafsi.taken_by)
                ),
                RafsiAvailability::ExperimentalTaken => format!(
                    "experimental-taken{}",
                    format_taken_rafsi_sources(&rafsi.taken_by)
                ),
            };
            format!("{}:{status}", rafsi.form)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[requires(true)]
#[ensures(true)]
fn format_taken_rafsi_sources(sources: &[String]) -> String {
    if sources.is_empty() {
        String::new()
    } else {
        format!("({})", sources.join("/"))
    }
}

#[requires(!value.is_empty())]
#[ensures(!ret.is_empty())]
fn trim_float(value: &str) -> String {
    let trimmed = value.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

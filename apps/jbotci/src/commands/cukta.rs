use super::super::*;

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_cukta<WOut: Write, WErr: Write>(
    input: CuktaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<CliStatus> {
    validate_cukta_input(&input)?;
    let request = cukta_request_from_input(&input)?;
    let site = embedded_cll_site().map_err(|error| anyhow!(error.to_string()))?;
    let mut tool_context = tool_context;
    let rendered = match &request {
        CuktaRequest::Search {
            mode: CuktaSearchMode::Meaning,
            query,
            count,
            targets,
        } => {
            let output = match run_semantic_cukta(
                tool_context.as_deref_mut(),
                site,
                query,
                *count,
                *targets,
            ) {
                Ok(output) => output,
                Err(error) => {
                    writeln!(stderr, "{error}")?;
                    return Ok(CliStatus::InvalidInput);
                }
            };
            render_search_output(&output, input.format.into(), CllLinkRenderMode::Plain)
        }
        _ => match render_cukta_request(
            site,
            &request,
            input.format.into(),
            CllLinkRenderMode::Plain,
        ) {
            Ok(rendered) => rendered,
            Err(CllError::SemanticSearchDisabled) => {
                writeln!(stderr, "{}", CllError::SemanticSearchDisabled)?;
                return Ok(CliStatus::InvalidInput);
            }
            Err(error) => return Err(anyhow!(error.to_string())),
        },
    };
    write!(stdout, "{rendered}")?;
    if !rendered.ends_with('\n') {
        writeln!(stdout)?;
    }
    Ok(CliStatus::Success)
}

#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_semantic_cukta(
    tool_context: Option<&mut ToolExecutionContext<'_>>,
    site: &jbotci_cll::CllSite,
    query: &str,
    count: usize,
    targets: CuktaTargetFilter,
) -> Result<jbotci_cll::CuktaSearchOutput> {
    let chunks = jbotci_cll::cll_search_all_chunks(site);
    if let Some(context) = tool_context
        && let Some(service) = context.embedding_search()?
    {
        return service
            .semantic_cukta_output(chunks, query, count, targets)
            .map_err(|error| anyhow!(error.to_string()));
    }
    let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
    let mut backend = load_backend_for_search(DEFAULT_MODEL_KEY, None)
        .map_err(|error| anyhow!(error.to_string()))?;
    semantic_cukta_output(
        &mut backend,
        chunks,
        query,
        count,
        targets,
        &index_root,
        DEFAULT_MODEL_KEY,
    )
    .map_err(|error| anyhow!(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_cukta_input(input: &CuktaInput) -> Result<()> {
    if input.count == Some(0) {
        bail!("`--count` must be greater than 0");
    }
    let request_mode_count = usize::from(input.toc)
        + usize::from(input.section.is_some())
        + usize::from(input.example.is_some())
        + usize::from(input.valsi.is_some())
        + usize::from(!input.query.is_empty());
    if request_mode_count > 1 {
        bail!(
            "Choose only one cukta mode: --toc, --section, --example, --valsi, or a positional query."
        );
    }
    if !input.targets.is_empty()
        || input.target_sections
        || input.target_paragraphs
        || input.target_examples
    {
        let _ = cukta_target_filter_from_input(input)?;
        if input.toc || input.section.is_some() || input.example.is_some() {
            bail!("Cukta target filters are only valid with search modes.");
        }
    }
    if request_mode_count == 0 {
        return Ok(());
    }
    if let Some(valsi) = &input.valsi
        && valsi.trim().is_empty()
    {
        bail!("`--valsi` requires a non-empty query.");
    }
    if let Some(section) = &input.section
        && section.trim().is_empty()
    {
        bail!("`--section` requires a non-empty reference.");
    }
    if let Some(example) = &input.example
        && example.trim().is_empty()
    {
        bail!("`--example` requires a non-empty reference.");
    }
    if !input.query.is_empty() && joined_query_text(&input.query).trim().is_empty() {
        bail!("cukta query text must be non-empty.");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn cukta_request_from_input(input: &CuktaInput) -> Result<CuktaRequest> {
    if input.toc {
        return Ok(CuktaRequest::Toc);
    }
    if let Some(reference) = &input.section {
        return Ok(CuktaRequest::Section {
            reference: reference.trim().to_owned(),
        });
    }
    if let Some(reference) = &input.example {
        return Ok(CuktaRequest::Example {
            reference: reference.trim().to_owned(),
        });
    }
    if let Some(query) = &input.valsi {
        return Ok(CuktaRequest::Search {
            mode: CuktaSearchMode::Word,
            query: query.trim().to_owned(),
            count: input.count.unwrap_or(DEFAULT_CUKTA_CLI_RESULT_COUNT),
            targets: cukta_target_filter_from_input(input)?,
        });
    }
    if !input.query.is_empty() {
        return Ok(CuktaRequest::Search {
            mode: CuktaSearchMode::Meaning,
            query: joined_query_text(&input.query).trim().to_owned(),
            count: input.count.unwrap_or(DEFAULT_CUKTA_CLI_RESULT_COUNT),
            targets: cukta_target_filter_from_input(input)?,
        });
    }
    Ok(CuktaRequest::Section {
        reference: jbotci_cll::DEFAULT_CUKTA_SECTION_ID.to_owned(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn cukta_target_filter_from_input(input: &CuktaInput) -> Result<CuktaTargetFilter> {
    let mut explicit = input.target_sections || input.target_paragraphs || input.target_examples;
    let mut sections = input.target_sections;
    let mut paragraphs = input.target_paragraphs;
    let mut examples = input.target_examples;
    for raw_target in &input.targets {
        for target in raw_target.split(',') {
            match target.trim().to_ascii_lowercase().as_str() {
                "" => {}
                "section" | "sections" => {
                    explicit = true;
                    sections = true;
                }
                "paragraph" | "paragraphs" => {
                    explicit = true;
                    paragraphs = true;
                }
                "example" | "examples" => {
                    explicit = true;
                    examples = true;
                }
                other => {
                    bail!(
                        "Unknown cukta search target `{other}`. Use section, paragraph, or example."
                    );
                }
            }
        }
    }
    if !explicit {
        return Ok(CuktaTargetFilter::default());
    }
    if !(sections || paragraphs || examples) {
        bail!("Select at least one cukta search target.");
    }
    Ok(CuktaTargetFilter {
        sections,
        paragraphs,
        examples,
    })
}

#[requires(true)]
#[ensures(true)]
fn cukta_target_flags_present(input: &CuktaInput) -> bool {
    !input.targets.is_empty()
        || input.target_sections
        || input.target_paragraphs
        || input.target_examples
}

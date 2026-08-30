use super::*;

#[requires(true)]
#[ensures(true)]
pub fn main_entry() -> ExitCode {
    match run() {
        Ok(CliStatus::Success) => ExitCode::SUCCESS,
        Ok(CliStatus::Failure) => ExitCode::FAILURE,
        Ok(CliStatus::ValidMissing) => ExitCode::from(10),
        Ok(CliStatus::InvalidInput) => ExitCode::from(11),
        Err(error) => {
            eprintln!("jbotci: {error}");
            ExitCode::FAILURE
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn run() -> Result<CliStatus> {
    let cli = Cli::parse();
    if matches!(&cli.command, Command::Lsp { .. }) {
        if cli.benchmark.is_some() {
            bail!("`--benchmark` is not supported with lsp");
        }
        super::lsp::run()?;
        return Ok(CliStatus::Success);
    }
    suppress_llama_logs_for_cli();
    let color_policy = CliColorPolicy {
        stdout: stream_supports_ansi_color(concolor::Stream::Stdout),
        stderr: stream_supports_ansi_color(concolor::Stream::Stderr),
    };
    let output_terminal_width = stdout_terminal_width();
    let diagnostic_terminal_width = stderr_terminal_width();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let progress_policy = CliProgressPolicy::embedding_setup(stderr.is_terminal());
    run_cli_with_color_policy_and_terminal_widths_and_progress(
        cli,
        &mut stdout,
        &mut stderr,
        color_policy,
        diagnostic_terminal_width,
        output_terminal_width,
        progress_policy,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(super) fn run_cli<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_enabled: bool,
) -> Result<CliStatus> {
    run_cli_with_color_policy(cli, stdout, stderr, CliColorPolicy::same(color_enabled))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_with_color_policy<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
) -> Result<CliStatus> {
    run_cli_with_color_policy_and_width(
        cli,
        stdout,
        stderr,
        color_policy,
        DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
    )
}

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(super) fn run_cli_with_color_policy_and_width<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
) -> Result<CliStatus> {
    run_cli_with_color_policy_and_terminal_widths(
        cli,
        stdout,
        stderr,
        color_policy,
        diagnostic_terminal_width,
        None,
    )
}

#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_with_color_policy_and_terminal_widths<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
) -> Result<CliStatus> {
    run_cli_with_color_policy_and_terminal_widths_and_progress(
        cli,
        stdout,
        stderr,
        color_policy,
        diagnostic_terminal_width,
        output_terminal_width,
        CliProgressPolicy::disabled(),
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_with_color_policy_and_terminal_widths_and_progress<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
    progress_policy: CliProgressPolicy,
) -> Result<CliStatus> {
    let color_policy = color_policy.with_choice(cli.color);
    if let Some(iterations) = cli.benchmark {
        return run_cli_benchmark(
            cli.command,
            iterations,
            stdout,
            stderr,
            color_policy,
            diagnostic_terminal_width,
            output_terminal_width,
        );
    }
    run_cli_command(
        cli.command,
        stdout,
        stderr,
        color_policy,
        diagnostic_terminal_width,
        output_terminal_width,
        progress_policy,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_benchmark<WOut: Write, WErr: Write>(
    command: Command,
    iterations: NonZeroUsize,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
) -> Result<CliStatus> {
    validate_benchmark_command(&command)?;
    let stdin_text = benchmark_stdin_text(&command)?;
    let mut measurement = BenchmarkMeasurement::start(iterations);
    for _ in 0..iterations.get() {
        let iteration_start = std::time::Instant::now();
        let status = run_cli_command(
            command.clone(),
            stdout,
            stderr,
            color_policy,
            diagnostic_terminal_width,
            output_terminal_width,
            CliProgressPolicy::disabled(),
            stdin_text.as_deref(),
        )?;
        measurement.record_iteration(iteration_start.elapsed(), status);
    }
    let report = measurement.finish();
    stderr.write_all(report.render().as_bytes())?;
    Ok(report.final_status())
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_command<WOut: Write, WErr: Write>(
    command: Command,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
    progress_policy: CliProgressPolicy,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    run_cli_command_with_tool_context(
        command,
        stdout,
        stderr,
        color_policy,
        diagnostic_terminal_width,
        output_terminal_width,
        progress_policy,
        stdin_text,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(super) fn run_cli_command_with_tool_context<WOut: Write, WErr: Write>(
    command: Command,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
    progress_policy: CliProgressPolicy,
    stdin_text: Option<&str>,
    mut tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<CliStatus> {
    match command {
        Command::Vlasei(input) => run_vlasei(
            input,
            stdout,
            stderr,
            color_policy,
            diagnostic_terminal_width,
            stdin_text,
        ),
        Command::Vlatai(input) => {
            run_vlatai(input, stdout, color_policy, diagnostic_terminal_width)
        }
        Command::Gentufa(mut input) => {
            let glyphs = cli_glyph_style(input.ascii);
            let diagnostic_detail = cli_diagnostic_detail(input.detailed_errors);
            let trace_limit = input.trace_limit.unwrap_or(DEFAULT_TRACE_LIMIT);
            let trace_limit_present = input.trace_limit.is_some();
            if trace_limit == 0 {
                bail!("--trace-limit must be greater than 0");
            }
            let requested_trace_phase = input.trace_phase.map(TracePhase::from);
            normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
            validate_trace_controls(
                &input.trace,
                new!(CliTraceValidation {
                    command_name: "gentufa",
                    trace_phase: requested_trace_phase,
                    trace_limit_present,
                    trace_list: input.trace_list,
                    supports_morphology: true,
                    supports_syntax: true,
                }),
            )?;
            if input.trace_list {
                write_trace_filter_list(
                    stdout,
                    requested_trace_phase.unwrap_or(TracePhase::Syntax),
                    true,
                    true,
                )?;
                return Ok(CliStatus::Success);
            }
            run_gentufa(
                input,
                stdout,
                stderr,
                color_policy,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
                CliTraceConfig {
                    phase: requested_trace_phase.unwrap_or(TracePhase::Syntax),
                    limit: trace_limit,
                },
                stdin_text,
            )
        }
        Command::Mulgau(input) => {
            validate_trace_controls_for_unsupported_command(
                "mulgau",
                &input.trace,
                None,
                false,
                false,
            )?;
            let _ = input.read_text_with_stdin(stdin_text)?;
            command_not_implemented("mulgau")?;
            Ok(CliStatus::Success)
        }
        Command::Vlacku(input) => {
            let glyphs = cli_glyph_style(input.ascii);
            run_vlacku(
                input,
                stdout,
                stderr,
                color_policy.stdout,
                glyphs,
                output_terminal_width,
                tool_context.as_deref_mut(),
            )
        }
        Command::Jvozba(input) => run_jvozba(input, stdout, color_policy.stdout),
        Command::Gimfihi(input) => run_gimfihi(input, stdout),
        Command::Cukta(input) => run_cukta(input, stdout, stderr, tool_context.as_deref_mut()),
        Command::Zbasu(input) => {
            validate_trace_controls_for_unsupported_command(
                "zbasu",
                &input.trace,
                None,
                false,
                false,
            )?;
            let _ = input.read_text_with_stdin(stdin_text)?;
            command_not_implemented("zbasu")?;
            Ok(CliStatus::Success)
        }
        Command::Setup(input) => run_setup(input, stdout, progress_policy),
        Command::Lsp { .. } => bail!("lsp requires the process standard input/output streams"),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_benchmark_command(command: &Command) -> Result<()> {
    if command_supports_benchmark(command) {
        Ok(())
    } else {
        bail!("`--benchmark` is only supported with vlasei, gentufa, vlacku, and cukta")
    }
}

#[requires(true)]
#[ensures(true)]
fn command_supports_benchmark(command: &Command) -> bool {
    matches!(
        command,
        Command::Vlasei(_) | Command::Gentufa(_) | Command::Vlacku(_) | Command::Cukta(_)
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn benchmark_stdin_text(command: &Command) -> Result<Option<String>> {
    if benchmark_command_reads_stdin(command) {
        read_text_input(None, &[], None).map(Some)
    } else {
        Ok(None)
    }
}

#[requires(true)]
#[ensures(true)]
fn benchmark_command_reads_stdin(command: &Command) -> bool {
    match command {
        Command::Vlasei(input) => vlasei_input_reads_stdin(input),
        Command::Gentufa(input) => gentufa_input_reads_stdin(input),
        _ => false,
    }
}

#[requires(true)]
#[ensures(input.file.is_some() -> !ret)]
fn vlasei_input_reads_stdin(input: &VlaseiInput) -> bool {
    if input.trace_list {
        return false;
    }
    trace_text_input_reads_stdin(&input.file, &input.text, &input.trace)
}

#[requires(true)]
#[ensures(input.file.is_some() -> !ret)]
fn gentufa_input_reads_stdin(input: &GentufaInput) -> bool {
    if input.trace_list {
        return false;
    }
    trace_text_input_reads_stdin(&input.file, &input.text, &input.trace)
}

#[requires(true)]
#[ensures(file.is_some() -> !ret)]
fn trace_text_input_reads_stdin(
    file: &Option<PathBuf>,
    text: &[String],
    trace: &Option<Option<String>>,
) -> bool {
    let mut normalized_trace = trace.clone();
    let mut normalized_text = text.to_owned();
    normalize_trace_text_input(&mut normalized_trace, file, &mut normalized_text);
    file.is_none() && normalized_text.is_empty()
}

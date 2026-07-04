use super::super::*;

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_jvozba<WOut: Write>(
    input: JvozbaInput,
    stdout: &mut WOut,
    color: bool,
) -> Result<CliStatus> {
    let mode = if input.cmevla {
        JvozbaMode::Cmevla
    } else {
        JvozbaMode::Lujvo
    };
    let result =
        build_best_jvozba_detailed(mode, jbotci_dictionary_data::english(), &input.sources)
            .map_err(|message| anyhow!(message))?;
    writeln!(stdout, "{}", render_jvozba_result(&result, color))?;
    Ok(CliStatus::Success)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_jvozba_result(result: &JvozbaBuildResult, color: bool) -> String {
    if !color || result.segments.is_empty() {
        return result.word.clone();
    }
    let mut rafsi_index = 0;
    let mut output = String::new();
    for segment in &result.segments {
        match segment.kind {
            JvozbaSegmentKind::Rafsi => {
                let segment_text = if rafsi_index % 2 == 0 {
                    green(&segment.text, true)
                } else {
                    magenta(&segment.text, true)
                };
                output.push_str(&segment_text);
                rafsi_index += 1;
            }
            JvozbaSegmentKind::Hyphen => output.push_str(&dark(&segment.text, true)),
        }
    }
    output
}

//! Gimfihi presentation: ranked candidates with comparable scores and source
//! contributions in a mobile-readable list, or the editable setup response.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_gimfihi::{
    CollisionKind, CollisionScope, GimfihiCandidate, RafsiAvailabilityData, RafsiClaimKind,
};

use super::markdown::{
    escape_in_scope, inline_code_in_scope, join_lines_in_scope, subtext_in_scope,
};
use super::{Pagination, RenderedResult, page_status};
use crate::discord::operations::GimfihiOutcome;
use crate::discord::request::{DiscordTool, GimfihiRequest, GimfihiScorer, GismuShape};

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.tool() == DiscordTool::Gimfihi))]
pub(crate) fn render_in_scope(
    outcome: &GimfihiOutcome,
    request: &GimfihiRequest,
    text_scope: &jbotci_output::budget::TextScope,
) -> RenderedResult {
    match outcome {
        GimfihiOutcome::Setup { preset, languages } => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Gimfihi,
                "gimfihi · setup".to_owned(),
                text_scope,
            );
            let mut lines = vec!["**No source words yet.**".to_owned()];
            match preset {
                Some(preset) => lines.push(text_scope.format(format_args!(
                    "Preset {} expects one word for each of: {}.",
                    inline_code_in_scope(preset.as_str(), text_scope), text_scope.join(languages.iter().map(|language| inline_code_in_scope(language, text_scope)), ", ")
                ))),
                None => lines.push(
                    "Choose a preset or give weighted sources; candidates are generated once every source has a word."
                        .to_owned(),
                ),
            }
            lines.push(subtext_in_scope(
                "Format: LANG[:WEIGHT]:WORD per record, separated by commas; WORD is Lojban letters or [narrow phonetic IPA] from the supported inventory. Example: eng:5:go, spa:3:[ir]",
             text_scope));
            rendered.body.push(join_lines_in_scope(lines, text_scope));
            rendered.notice = Some(subtext_in_scope(
                "Use the ⚙️ button to enter sources.",
                text_scope,
            ));
            rendered
        }
        GimfihiOutcome::Candidates {
            sources,
            winner,
            candidate_count,
            filtered_count,
            results,
        } => {
            let options = request.options;
            // Which scorer ranked these is part of what the result is, so it
            // is shown for both of them rather than only for the one that is
            // not the default: silence would be indistinguishable from a
            // result published before there was a choice.
            let mut status = text_scope.buffer_with(&text_scope.format(format_args!(
                "gimfihi · {} · {}",
                page_status(results),
                options.scorer.as_str()
            )));
            if let Some(preset) = options.preset {
                status.push_str(&text_scope.format(format_args!(" · {}", preset.as_str())));
            }
            let mut rendered =
                RenderedResult::new_in_scope(DiscordTool::Gimfihi, status, text_scope);
            rendered.pagination = Some(Pagination::of(results));
            let mut lines = Vec::new();
            let source_text = text_scope.join(
                sources.iter().map(|source| match &source.ipa {
                    Some(ipa) => text_scope.format(format_args!(
                        "{}:[{}]→{} ×{}",
                        source.language, ipa, source.word, source.weight
                    )),
                    None => text_scope.format(format_args!(
                        "{}:{} ×{}",
                        source.language, source.word, source.weight
                    )),
                }),
                " · ",
            );
            lines.push(subtext_in_scope(
                &text_scope.format(format_args!("sources: {source_text}")),
                text_scope,
            ));
            let shapes = if options.shapes.is_empty() {
                "ccvcv+cvccv".to_owned()
            } else {
                text_scope.join(
                    options.shapes.shapes().iter().map(|shape| match shape {
                        GismuShape::Ccvcv => "ccvcv",
                        GismuShape::Cvccv => "cvccv",
                    }),
                    "+",
                )
            };
            let collisions = match options.collisions {
                CollisionScope::All => "checked against all gismu",
                CollisionScope::Official => "checked against official gismu only",
                CollisionScope::None => "NOT checked (collision checking off)",
            };
            let mut settings = vec![
                text_scope.format(format_args!("shapes {shapes}")),
                collisions.to_owned(),
            ];
            if options.show_collisions {
                settings.push("colliding candidates shown".to_owned());
            }
            // The phonetic scorer scores every letter whatever the setting
            // says, so this line reports what actually happened rather than
            // what was ticked.
            if options.all_letters || options.scorer == GimfihiScorer::Phonetic {
                settings.push("all letters scored".to_owned());
            }
            if options.require_free_short_rafsi {
                settings.push("free short rafsi required".to_owned());
            }
            // The scorer counts every candidate it filtered, so this total is
            // the real one rather than the size of what was fetched.
            lines.push(subtext_in_scope(
                &text_scope.format(format_args!(
                    "{} of {} passing ({} valid) · {}",
                    results
                        .range()
                        .map(|(first, last)| text_scope.format(format_args!("{first}-{last}")))
                        .unwrap_or_else(|| "none".to_owned()),
                    filtered_count,
                    candidate_count,
                    text_scope.join(&settings, " · ")
                )),
                text_scope,
            ));
            if results.items.is_empty() {
                lines.push("**No gismu candidates matched the selected filters.**".to_owned());
            }
            for candidate in &results.items {
                lines.push(candidate_line_in_scope(
                    candidate,
                    winner.as_deref(),
                    options.collisions,
                    text_scope,
                ));
            }
            rendered.body.push(join_lines_in_scope(lines, text_scope));
            rendered
        }
    }
}

/// One candidate: rank, word, score, rafsi standing and per-source
/// contributions, each on its own short line so nothing needs alignment.
#[requires(!candidate.word.is_empty())]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn candidate_line_in_scope(
    candidate: &GimfihiCandidate,
    winner: Option<&str>,
    scope: CollisionScope,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    let mut head = text_scope.buffer_with(&text_scope.format(format_args!(
        "**{}** — score {}",
        escape_in_scope(&candidate.word, text_scope),
        format_score_in_scope(candidate.score, text_scope)
    )));
    if winner == Some(candidate.word.as_str()) {
        head.push_str(" · best");
    }
    if let Some(collision) = &candidate.collision {
        let kind = match collision.kind {
            CollisionKind::Identical => text_scope.format(format_args!(
                "identical to existing {}",
                collision.existing_word_type.as_str()
            )),
            CollisionKind::FinalVowel => text_scope.format(format_args!(
                "collides with {} (final vowel)",
                collision.existing_word
            )),
            CollisionKind::SimilarConsonant => text_scope.format(format_args!(
                "collides with {} (similar consonant)",
                collision.existing_word
            )),
        };
        head.push_str(
            &text_scope.format(format_args!(" · ⚠ {}", escape_in_scope(&kind, text_scope))),
        );
    }
    let mut details = Vec::new();
    let rafsi = candidate
        .rafsi()
        .iter()
        .map(|rafsi| {
            let standing = match rafsi.availability.as_data() {
                data!(RafsiAvailability::Free) => "free".to_owned(),
                data!(RafsiAvailability::Taken { kind, words }) => text_scope.format(format_args!(
                    "{} by {}",
                    match kind {
                        RafsiClaimKind::Official => "taken",
                        RafsiClaimKind::Experimental => "taken (experimental)",
                    },
                    text_scope.join(words.iter().map(String::as_str), "/")
                )),
            };
            text_scope.format(format_args!(
                "{} {}",
                inline_code_in_scope(&rafsi.form, text_scope),
                escape_in_scope(&standing, text_scope)
            ))
        })
        .collect::<Vec<_>>();
    if !rafsi.is_empty() {
        details.push(text_scope.format(format_args!("rafsi: {}", text_scope.join(&rafsi, ", "))));
    } else if scope != CollisionScope::None {
        details.push("rafsi: none possible".to_owned());
    }
    let contributions = candidate
        .source_scores
        .iter()
        .map(|score| {
            text_scope.format(format_args!(
                "{} {}",
                score.language,
                format_score_in_scope(score.weighted_score, text_scope)
            ))
        })
        .collect::<Vec<_>>();
    if !contributions.is_empty() {
        details.push(text_scope.format(format_args!(
            "from: {}",
            escape_in_scope(&text_scope.join(&contributions, ", "), text_scope)
        )));
    }
    if details.is_empty() {
        head.into_piece()
    } else {
        text_scope.format(format_args!(
            "{head}\n-# {}",
            text_scope.join(&details, " · ")
        ))
    }
}

#[requires(score.is_finite())]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn format_score_in_scope(score: f64, text_scope: &jbotci_output::budget::TextScope) -> String {
    let text = text_scope.format(format_args!("{score:.3}"));
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render(outcome: &GimfihiOutcome, request: &GimfihiRequest) -> RenderedResult {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_in_scope(outcome, request, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::operations::PagedResults;
    use crate::discord::request::{GimfihiOptions, PageNumber, SourceText};
    use jbotci_gimfihi::{
        GimfihiPreset, GimfihiRequest as SharedRequest, GimfihiScorer, compose_gismu,
        parse_source_spec,
    };

    #[requires(true)]
    #[ensures(true)]
    fn discord_request(sources: Option<&str>, preset: Option<GimfihiPreset>) -> GimfihiRequest {
        GimfihiRequest {
            sources: sources.map(|text| SourceText::new(text).expect("text")),
            options: GimfihiOptions {
                preset,
                ..GimfihiOptions::default()
            },
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn setup_response_explains_the_preset_and_format() {
        let rendered = render(
            &GimfihiOutcome::Setup {
                preset: Some(GimfihiPreset::Ilmen6),
                languages: vec!["eng".to_owned(), "cmn".to_owned()],
            },
            &discord_request(None, Some(GimfihiPreset::Ilmen6)),
        );
        assert_eq!(rendered.status.text, "gimfihi · setup");
        let body = rendered.body.join("\n");
        assert!(body.contains("No source words yet"), "{body}");
        assert!(
            body.contains("`ilmen6`") && body.contains("`eng`"),
            "{body}"
        );
        assert!(!body.contains("score"), "no invented candidates: {body}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn candidates_render_scores_rafsi_and_collision_scope() {
        let sources = ["eng:5:go", "spa:3:[ir]"]
            .iter()
            .map(|spec| parse_source_spec(spec).expect("spec"))
            .collect::<Vec<_>>();
        let output = compose_gismu(
            jbotci_dictionary_data::english(),
            &SharedRequest {
                scorer: GimfihiScorer::Classic,
                phonetic_parameters: Default::default(),
                preset: None,
                sources,
                shapes: jbotci_gimfihi::default_shapes(),
                all_letters: false,
                check_collisions: CollisionScope::None,
                show_collisions: false,
                require_free_short_rafsi: false,
                skip: 0,
                count: 126,
                highlight: None,
            },
        )
        .expect("candidates");
        let results = PagedResults::from_all(output.candidates, PageNumber::first()).expect("page");
        let mut request = discord_request(Some("eng:5:go, spa:3:[ir]"), None);
        request.options.collisions = CollisionScope::None;
        let rendered = render(
            &GimfihiOutcome::Candidates {
                sources: output.resolved_sources,
                winner: output.winner,
                candidate_count: output.candidate_count,
                filtered_count: output.filtered_count,
                results,
            },
            &request,
        );
        assert!(
            rendered.status.text.starts_with("gimfihi · 1-5 of "),
            "{}",
            rendered.status.text
        );
        let body = rendered.body.join("\n");
        assert!(
            body.contains("NOT checked"),
            "collision scope is stated: {body}"
        );
        assert!(body.contains("score"), "{body}");
        assert!(body.contains("from: eng"), "source contributions: {body}");
        assert!(!body.contains("mark  gismu"), "no console table header");
    }
}

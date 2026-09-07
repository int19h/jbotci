//! "Open in app" links for the tools that have a web page.
//!
//! A link is the published request expressed as shared web route state, so it
//! opens the same task with the same input in a browser that has never spoken
//! to this server. Gentufa, Vlacku, Cukta and Gimfi'i have pages; Vlasei,
//! Vlatai and Jvozba do not, and their forms carry no link component and no
//! substitute.
//!
//! The route codec transports text exactly, so the link's length grows with
//! the input: percent-encoding costs up to nine characters per character
//! outside ASCII. A request whose link would not fit the modal is refused
//! before anything is published, because a published result must always be
//! able to reopen with its link. The bound is jbotci's own conservative
//! budget for one modal text component: Discord's component reference
//! documents Text Display content as a string without a numeric maximum, so
//! this is a chosen limit, not a platform-documented one.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_cll::{cll_lookup_example, cll_resolve_example_reference, embedded_cll_site};
use jbotci_web_core::{
    CuktaSearchTarget, CuktaWebMode, CuktaWebSearchState, CuktaWebState, CuktaWebView,
    GentufaWebState, GentufaWebViewMode, GimfihiWebState, VlackuWebMode, VlackuWebState, WebRoute,
    gimfihi_web_source_from_record, web_route_url,
};

use super::operations::{CUKTA_FETCH_COUNT, GIMFIHI_RECORD_SEPARATORS, VLACKU_FETCH_COUNT};
use super::request::{
    CuktaMode, CuktaRequest, CuktaResultKind, DiscordRequest, DiscordTool, GentufaRequest,
    GentufaTextView, GimfihiRequest, VlackuMode, VlackuRequest, utf16_len,
};

/// Most units one modal text component may carry, this application's own
/// conservative budget rather than a platform-documented maximum.
pub(crate) const APP_LINK_BUDGET_UNITS: usize = 4000;
/// The label of the link, and the beginning of the rendered component.
pub(crate) const APP_LINK_LABEL: &str = "Open in app";

/// A request whose app link does not fit the modal budget. Such a request is
/// refused before publication: a published result must always reopen, and its
/// form must always be able to offer the exact-state link.
#[invariant(*units > *budget)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppLinkTooLarge {
    pub(crate) tool: DiscordTool,
    pub(crate) units: usize,
    pub(crate) budget: usize,
    /// The tool's own page, offered as a place to enter the input. It does
    /// not carry the refused state and never claims to.
    pub(crate) tool_page: String,
}

impl fmt::Display for AppLinkTooLarge {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "this input needs a {}-character app link, above the {}-character modal budget; open {} in the app and enter it there: {}",
            self.units, self.budget, self.tool, self.tool_page
        )
    }
}

impl std::error::Error for AppLinkTooLarge {}

/// The rendered link component for `request`, or `None` when the tool has no
/// web page. `base_url` is the public root of the web app, without a trailing
/// slash (for example `https://jbotci.app`).
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|link| link.as_ref().is_none_or(|link| utf16_len(link) <= APP_LINK_BUDGET_UNITS)) || ret.is_err())]
#[ensures(ret.as_ref().is_ok_and(|link| link.is_some() == request.tool().has_web_page()) || ret.is_err())]
pub(crate) fn app_link(
    request: &DiscordRequest,
    base_url: &str,
) -> Result<Option<String>, AppLinkTooLarge> {
    let Some(target) = link_target(request) else {
        return Ok(None);
    };
    let url = target.url(base_url);
    let component = format!("[{APP_LINK_LABEL}]({url})");
    let units = utf16_len(&component);
    if units > APP_LINK_BUDGET_UNITS {
        return Err(new!(AppLinkTooLarge {
            tool: request.tool(),
            units,
            budget: APP_LINK_BUDGET_UNITS,
            tool_page: tool_page_url(request.tool(), base_url),
        }));
    }
    Ok(Some(component))
}

/// Where a tool's page lives, with no state attached.
#[requires(true)]
#[ensures(ret.starts_with(base_url) || base_url.is_empty())]
pub(crate) fn tool_page_url(tool: DiscordTool, base_url: &str) -> String {
    let prefix = base_url.trim_end_matches('/');
    match tool {
        DiscordTool::Gentufa => format!("{prefix}/gentufa"),
        DiscordTool::Vlacku => format!("{prefix}/vlacku"),
        DiscordTool::Cukta => format!("{prefix}/cukta"),
        DiscordTool::Gimfihi => format!("{prefix}/gimfihi"),
        DiscordTool::Vlasei | DiscordTool::Vlatai | DiscordTool::Jvozba => prefix.to_owned(),
    }
}

/// A web target: shared route state, plus the anchor a Cukta example needs
/// (the book renders examples inside their section, so the example's route is
/// its section's route with the example's anchor).
#[invariant(anchor.as_ref().is_none_or(|anchor| !anchor.is_empty()))]
#[derive(Debug, Clone, PartialEq)]
struct LinkTarget {
    route: WebRoute,
    anchor: Option<String>,
}

impl LinkTarget {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn url(&self, base_url: &str) -> String {
        let mut url = format!(
            "{}{}",
            base_url.trim_end_matches('/'),
            web_route_url("", &self.route)
        );
        if let Some(anchor) = &self.anchor {
            url.push('#');
            url.push_str(anchor);
        }
        url
    }
}

/// The web state a published request corresponds to, or `None` for the tools
/// without a page. No request is ever mapped onto another tool's page.
#[requires(true)]
#[ensures(ret.is_some() == request.tool().has_web_page())]
fn link_target(request: &DiscordRequest) -> Option<LinkTarget> {
    let route = match request {
        DiscordRequest::Gentufa(request) => WebRoute::Gentufa(gentufa_state(request)),
        DiscordRequest::Vlacku(request) => WebRoute::Vlacku(vlacku_state(request)),
        DiscordRequest::Cukta(request) => return Some(cukta_target(request)),
        DiscordRequest::Gimfihi(request) => WebRoute::Gimfihi(gimfihi_state(request)),
        DiscordRequest::Vlasei(_) | DiscordRequest::Vlatai(_) | DiscordRequest::Jvozba(_) => {
            return None;
        }
    };
    Some(new!(LinkTarget {
        route,
        anchor: None,
    }))
}

#[requires(true)]
#[ensures(ret.text == request.text.as_str())]
fn gentufa_state(request: &GentufaRequest) -> GentufaWebState {
    let options = request.options;
    GentufaWebState {
        text: request.text.as_str().to_owned(),
        // An explicitly cleared dialect stays explicit, so the page opens with
        // the field empty rather than restoring a remembered formula.
        dialect: request
            .dialect
            .as_ref()
            .map(|dialect| dialect.as_str().to_owned()),
        view_mode: match options.view {
            GentufaTextView::Brackets => GentufaWebViewMode::Blocks,
            GentufaTextView::Tree => GentufaWebViewMode::Tree,
        },
        show_elided: options.show_elided,
        show_glosses: options.show_glosses,
        show_compounds: options.show_compounds,
    }
}

#[requires(true)]
#[ensures(ret.query == request.query.as_str())]
fn vlacku_state(request: &VlackuRequest) -> VlackuWebState {
    let options = request.options;
    VlackuWebState {
        // The page has no separate lujvo search: a lujvo query is the same
        // word lookup, which shows the entry and its parts.
        mode: match options.mode {
            VlackuMode::Word | VlackuMode::Lujvo => VlackuWebMode::Word,
            VlackuMode::Rafsi => VlackuWebMode::Rafsi,
            VlackuMode::Sound => VlackuWebMode::Sound,
            VlackuMode::Meaning => VlackuWebMode::Meaning,
        },
        query: request.query.as_str().to_owned(),
        // Everything Discord could page through is on the page at once.
        count: VLACKU_FETCH_COUNT,
        word_types: options
            .word_types
            .iter()
            .map(|word_type| word_type.filter_value().to_owned())
            .collect(),
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_target(request: &CuktaRequest) -> LinkTarget {
    let options = request.options;
    let query = request
        .query
        .as_ref()
        .map(|query| query.as_str().to_owned())
        .unwrap_or_default();
    let targets = CuktaResultKind::ALL
        .iter()
        .filter(|kind| options.kinds.contains(**kind))
        .map(|kind| match kind {
            CuktaResultKind::Section => CuktaSearchTarget::Section,
            CuktaResultKind::Paragraph => CuktaSearchTarget::Paragraph,
            CuktaResultKind::Example => CuktaSearchTarget::Example,
        })
        .collect::<Vec<_>>();
    let view = match options.mode {
        CuktaMode::Meaning | CuktaMode::Word => CuktaWebView::Search(CuktaWebSearchState {
            mode: if options.mode == CuktaMode::Meaning {
                CuktaWebMode::Meaning
            } else {
                CuktaWebMode::Word
            },
            query,
            count: CUKTA_FETCH_COUNT,
            targets,
        }),
        CuktaMode::Section => CuktaWebView::Section { reference: query },
        CuktaMode::Example => {
            // The book has no example route: an example is read inside its
            // own section, at its own anchor.
            let located = embedded_cll_site().ok().and_then(|site| {
                let example_id = cll_resolve_example_reference(site, query.trim())?;
                let example = cll_lookup_example(site, &example_id)?;
                Some((
                    example.reference.section_id.clone(),
                    example.anchor_id.clone(),
                ))
            });
            return match located {
                Some((section_id, anchor)) => new!(LinkTarget {
                    route: WebRoute::Cukta(CuktaWebState {
                        view: CuktaWebView::Section {
                            reference: section_id,
                        },
                    }),
                    anchor: Some(anchor),
                }),
                // An unresolved reference has no page of its own; the link
                // opens the book where the reader can search for it.
                None => new!(LinkTarget {
                    route: WebRoute::Cukta(CuktaWebState {
                        view: CuktaWebView::Search(CuktaWebSearchState {
                            mode: CuktaWebMode::Word,
                            query,
                            count: CUKTA_FETCH_COUNT,
                            targets,
                        }),
                    }),
                    anchor: None,
                }),
            };
        }
        // The book's own contents are the page's navigation, shown beside the
        // section it opens on.
        CuktaMode::Contents => CuktaWebState::default().view,
    };
    new!(LinkTarget {
        route: WebRoute::Cukta(CuktaWebState { view }),
        anchor: None,
    })
}

#[requires(true)]
#[ensures(ret.preset == request.options.preset)]
fn gimfihi_state(request: &GimfihiRequest) -> GimfihiWebState {
    let options = request.options;
    let defaults = GimfihiWebState::default();
    let sources = request
        .sources
        .as_ref()
        .map(|sources| {
            sources
                .as_str()
                .split(GIMFIHI_RECORD_SEPARATORS)
                .map(str::trim)
                .filter(|record| !record.is_empty())
                // The record travels through the same codec the route uses,
                // so the form opens on the language, weight and word as they
                // were written, whether or not they resolve to a usable
                // source. Meaning is applied when the sources are resolved,
                // not when they are transported.
                .map(gimfihi_web_source_from_record)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    GimfihiWebState {
        preset: options.preset,
        scorer: defaults.scorer,
        sources,
        shapes: options.shapes.shapes(),
        all_letters: options.all_letters,
        check_collisions: options.collisions,
        show_collisions: options.show_collisions,
        require_free_short_rafsi: options.require_free_short_rafsi,
        count: defaults.count,
        highlight: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::{
        CuktaOptions, CuktaResultKindSet, GentufaOptions, GimfihiOptions, GismuShape,
        GismuShapeSet, JvozbaOptions, JvozbaRequest, MAX_SOURCE_UNITS, PageNumber, SourceText,
        VlackuOptions, VlackuWordType, VlackuWordTypeSet, VlaseiOptions, VlaseiRequest,
    };
    use jbotci_gimfihi::{CollisionScope, GimfihiPreset};
    use jbotci_web_core::{
        gimfihi_web_source_from_record, parse_cukta_web_route, parse_gentufa_web_route,
        parse_gimfihi_web_route, parse_vlacku_web_route,
    };

    const BASE: &str = "https://jbotci.app";

    #[requires(true)]
    #[ensures(true)]
    fn gentufa(text: &str, dialect: Option<&str>) -> DiscordRequest {
        DiscordRequest::Gentufa(GentufaRequest {
            text: SourceText::new(text).expect("text"),
            dialect: dialect.map(|dialect| SourceText::new(dialect).expect("dialect")),
            options: GentufaOptions {
                view: GentufaTextView::Tree,
                include_diagram: true,
                show_elided: true,
                show_compounds: false,
                show_glosses: true,
            },
        })
    }

    /// The link's URL, with the label and brackets stripped.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn url_of(request: &DiscordRequest) -> String {
        let component = app_link(request, BASE)
            .expect("within budget")
            .expect("a tool with a page");
        component
            .strip_prefix(&format!("[{APP_LINK_LABEL}]("))
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("a markdown link")
            .to_owned()
    }

    /// The URL's path and query, as the browser would hand them to the router.
    #[requires(true)]
    #[ensures(true)]
    fn route_parts(url: &str) -> (String, String) {
        let path = url.strip_prefix(BASE).expect("the configured base");
        match path.split_once('?') {
            Some((path, query)) => (path.to_owned(), query.to_owned()),
            None => (path.to_owned(), String::new()),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_links_reopen_the_exact_published_state() {
        let request = gentufa("  mi klama lo zarci  ", Some(""));
        let url = url_of(&request);
        let (path, query) = route_parts(&url);
        let state = parse_gentufa_web_route(&path, &query);
        assert_eq!(state.text, "  mi klama lo zarci  ", "{url}");
        assert_eq!(state.dialect.as_deref(), Some(""), "{url}");
        assert_eq!(state.view_mode, GentufaWebViewMode::Tree);
        assert!(state.show_elided && state.show_glosses && !state.show_compounds);
        // The diagram is a Discord attachment, not a page setting, and never
        // leaks into the link.
        assert!(!query.contains("diagram"), "{url}");

        let dialect = gentufa("mi klama", Some("(cbm)"));
        let (path, query) = route_parts(&url_of(&dialect));
        assert_eq!(
            parse_gentufa_web_route(&path, &query).dialect.as_deref(),
            Some("(cbm)")
        );
        let absent = gentufa("mi klama", None);
        let (path, query) = route_parts(&url_of(&absent));
        assert_eq!(parse_gentufa_web_route(&path, &query).dialect, None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tools_without_a_page_offer_no_link_at_all() {
        for request in [
            DiscordRequest::Vlasei(VlaseiRequest {
                text: SourceText::new("mi klama").expect("text"),
                dialect: None,
                options: VlaseiOptions::default(),
            }),
            DiscordRequest::Jvozba(JvozbaRequest {
                parts: SourceText::new("klama bajra").expect("text"),
                options: JvozbaOptions::default(),
            }),
        ] {
            assert_eq!(
                app_link(&request, BASE).expect("no bound applies"),
                None,
                "{:?} has no page",
                request.tool()
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn search_links_carry_query_mode_and_filters() {
        let vlacku = DiscordRequest::Vlacku(new!(VlackuRequest {
            query: SourceText::new("  go & come  ").expect("text"),
            options: VlackuOptions {
                mode: VlackuMode::Meaning,
                word_types: VlackuWordTypeSet::empty().with(VlackuWordType::Gismu),
                decompose_lujvo: true,
                show_etymology: false,
                page: PageNumber::new(3).expect("page"),
            },
        }));
        let (path, query) = route_parts(&url_of(&vlacku));
        let state = parse_vlacku_web_route(&path, &query);
        assert_eq!(state.query, "  go & come  ");
        assert_eq!(state.mode, VlackuWebMode::Meaning);
        assert_eq!(state.word_types, vec!["gismu".to_owned()]);
        assert_eq!(
            state.count, VLACKU_FETCH_COUNT,
            "the page shows every result Discord paged"
        );

        // A lujvo search is the same word lookup on the page.
        let lujvo = DiscordRequest::Vlacku(new!(VlackuRequest {
            query: SourceText::new("klabajra").expect("text"),
            options: VlackuOptions {
                mode: VlackuMode::Lujvo,
                ..VlackuOptions::default()
            },
        }));
        let (path, query) = route_parts(&url_of(&lujvo));
        let state = parse_vlacku_web_route(&path, &query);
        assert_eq!(state.mode, VlackuWebMode::Word);
        assert_eq!(state.query, "klabajra");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_links_open_sections_examples_searches_and_the_book() {
        let section = DiscordRequest::Cukta(CuktaRequest {
            query: Some(SourceText::new("4.6").expect("text")),
            options: CuktaOptions {
                mode: CuktaMode::Section,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        });
        let url = url_of(&section);
        assert!(
            url.starts_with("https://jbotci.app/cukta/section/"),
            "{url}"
        );

        let example = DiscordRequest::Cukta(CuktaRequest {
            query: Some(SourceText::new("4.27").expect("text")),
            options: CuktaOptions {
                mode: CuktaMode::Example,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        });
        let url = url_of(&example);
        let (before_hash, anchor) = url.split_once('#').expect("an example anchor");
        assert!(before_hash.contains("/cukta/section/"), "{url}");
        assert!(!anchor.is_empty(), "{url}");

        let search = DiscordRequest::Cukta(CuktaRequest {
            query: Some(SourceText::new("tanru").expect("text")),
            options: CuktaOptions {
                mode: CuktaMode::Word,
                kinds: CuktaResultKindSet::empty().with(CuktaResultKind::Example),
                page: PageNumber::first(),
            },
        });
        let (path, query) = route_parts(&url_of(&search));
        let CuktaWebView::Search(state) = parse_cukta_web_route(&path, &query).view else {
            panic!("a search route");
        };
        assert_eq!(state.mode, CuktaWebMode::Word);
        assert_eq!(state.query, "tanru");
        assert_eq!(state.targets, vec![CuktaSearchTarget::Example]);

        let contents = DiscordRequest::Cukta(CuktaRequest {
            query: None,
            options: CuktaOptions {
                mode: CuktaMode::Contents,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        });
        let url = url_of(&contents);
        assert!(
            url.starts_with("https://jbotci.app/cukta/section/"),
            "{url}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gimfihi_links_carry_sources_shapes_and_collision_settings() {
        let request = DiscordRequest::Gimfihi(GimfihiRequest {
            sources: Some(SourceText::new("eng:5:go, spa:3:[ir]").expect("text")),
            options: GimfihiOptions {
                preset: Some(GimfihiPreset::Ilmen6),
                shapes: GismuShapeSet::empty().with(GismuShape::Ccvcv),
                collisions: CollisionScope::Official,
                show_collisions: true,
                all_letters: true,
                require_free_short_rafsi: true,
                page: PageNumber::first(),
            },
        });
        let (path, query) = route_parts(&url_of(&request));
        let state = parse_gimfihi_web_route(&path, &query);
        assert_eq!(state.preset, Some(GimfihiPreset::Ilmen6));
        assert_eq!(state.sources.len(), 2);
        assert_eq!(state.sources[0].language, "eng");
        assert_eq!(state.sources[0].weight.as_deref(), Some("5"));
        assert_eq!(state.sources[0].word, "go");
        assert_eq!(state.shapes, vec![GismuShape::Ccvcv]);
        assert_eq!(state.check_collisions, CollisionScope::Official);
        assert!(state.show_collisions && state.all_letters && state.require_free_short_rafsi);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gimfihi_source_records_reach_the_form_as_they_were_written() {
        for record in [
            "eng:5:go",
            // A weight the source resolver rejects is still a record the
            // reader typed, and the form must show it back unchanged.
            "eng:0:klama",
            // IPA keeps its spacing and its case.
            "eng:5:[i r]",
            "eng:5:[ɪR]",
            "eng::go",
            // Colons and separators inside the word cannot grow prefixes.
            "eng:5:a:b:c",
            "eng:5:go+stop",
            "just-a-word",
            "ENG:5:GO",
        ] {
            let request = DiscordRequest::Gimfihi(GimfihiRequest {
                sources: Some(SourceText::new(record).expect("text")),
                options: GimfihiOptions::default(),
            });
            let (path, query) = route_parts(&url_of(&request));
            let state = parse_gimfihi_web_route(&path, &query);
            assert_eq!(
                state.sources,
                vec![gimfihi_web_source_from_record(record)],
                "{record}"
            );
        }

        // The reported corruption, spelled out: three fields, no prefix.
        let request = DiscordRequest::Gimfihi(GimfihiRequest {
            sources: Some(SourceText::new("eng:0:klama").expect("text")),
            options: GimfihiOptions::default(),
        });
        let (path, query) = route_parts(&url_of(&request));
        let source = parse_gimfihi_web_route(&path, &query)
            .sources
            .pop()
            .expect("one record");
        assert_eq!(source.language, "eng");
        assert_eq!(source.weight.as_deref(), Some("0"));
        assert_eq!(source.word, "klama");

        // Several records become several rows, each with its own values.
        let request = DiscordRequest::Gimfihi(GimfihiRequest {
            sources: Some(SourceText::new("eng:5:go, spa:3:[ir]\nfra::aller").expect("text")),
            options: GimfihiOptions::default(),
        });
        let (path, query) = route_parts(&url_of(&request));
        let state = parse_gimfihi_web_route(&path, &query);
        assert_eq!(
            state
                .sources
                .iter()
                .map(|source| (source.language.as_str(), source.word.as_str()))
                .collect::<Vec<_>>(),
            vec![("eng", "go"), ("spa", "[ir]"), ("fra", "aller")]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_input_whose_link_exceeds_the_budget_is_refused_with_its_numbers() {
        // Ordinary Latin prose of a few thousand characters links fine.
        let ascii = gentufa(&"mi klama ".repeat(430), None);
        assert!(app_link(&ascii, BASE).is_ok(), "3870 ASCII units fit");

        // A source at its own 4000-unit maximum cannot also carry a
        // 4000-unit link once the route and its options are counted, so that
        // request is refused rather than published without one.
        let maximal = gentufa(&"a".repeat(MAX_SOURCE_UNITS), None);
        let error = app_link(&maximal, BASE).expect_err("over the budget");
        assert!(error.units > APP_LINK_BUDGET_UNITS, "{}", error.units);

        // Text outside ASCII costs up to nine characters per character.
        let cyrillic = gentufa(&"клама ".repeat(600), None);
        let error = app_link(&cyrillic, BASE).expect_err("over the budget");
        assert_eq!(error.tool, DiscordTool::Gentufa);
        assert_eq!(error.budget, APP_LINK_BUDGET_UNITS);
        assert!(error.units > APP_LINK_BUDGET_UNITS);
        assert_eq!(error.tool_page, "https://jbotci.app/gentufa");
        let message = error.to_string();
        assert!(
            message.contains("above the 4000-character modal budget"),
            "{message}"
        );
        assert!(message.contains("https://jbotci.app/gentufa"), "{message}");
        assert!(
            !message.contains("klama"),
            "the refusal does not repeat the input"
        );

        // Every field shares one budget: either alone fits, both do not.
        let field = "mi klama ".repeat(230);
        assert!(
            app_link(&gentufa(&field, None), BASE).is_ok(),
            "the source alone fits"
        );
        assert!(
            app_link(&gentufa("mi", Some(&field)), BASE).is_ok(),
            "the dialect alone fits"
        );
        assert!(
            app_link(&gentufa(&field, Some(&field)), BASE).is_err(),
            "the fields are budgeted together"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_budget_is_measured_on_the_whole_component() {
        // Exactly at the budget and one unit over, measured through the real
        // encoder: the label, the brackets and every option parameter count,
        // not just the source text. Each further ASCII letter costs one unit.
        let probe = gentufa("a", None);
        let probe_units = utf16_len(
            &app_link(&probe, BASE)
                .expect("a short link")
                .expect("a link"),
        );
        let fill = APP_LINK_BUDGET_UNITS - probe_units + 1;
        assert!(
            fill < MAX_SOURCE_UNITS,
            "a maximal source cannot fit the link budget: {fill} of {MAX_SOURCE_UNITS}"
        );
        let exact = gentufa(&"a".repeat(fill), None);
        let component = app_link(&exact, BASE)
            .expect("exactly at the budget")
            .expect("a link");
        assert_eq!(utf16_len(&component), APP_LINK_BUDGET_UNITS);
        let over = gentufa(&"a".repeat(fill + 1), None);
        let error = app_link(&over, BASE).expect_err("one unit over");
        assert_eq!(error.units, APP_LINK_BUDGET_UNITS + 1);
    }
}

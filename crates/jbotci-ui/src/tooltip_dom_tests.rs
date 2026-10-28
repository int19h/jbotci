use super::*;

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn TooltipCardsDomApp() -> Element {
    let missing = dictionary_tooltip_for_word("", "klabajra").unwrap();
    let known = dictionary_tooltip_for_word("", "brivla").unwrap();
    let reference = new!(ReferenceTooltip {
        card: Some(missing.clone()),
        missing_word: None,
        highlighted_places: Vec::new(),
        definition: Vec::new(),
        notes: Vec::new(),
        rows: Vec::new(),
    });
    rsx! {
        div { id: "missing", { render_dictionary_tooltip(&missing, false, "", GentufaScript::Latin) } }
        div { id: "known", { render_dictionary_tooltip(&known, false, "", GentufaScript::Latin) } }
        div { id: "reference", { render_reference_tooltip(&reference, "", GentufaScript::Latin) } }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn shared_hover_renderer_adds_ordered_components_only_for_missing_lujvo() {
    let mut dom = VirtualDom::new(TooltipCardsDomApp);
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    let missing_start = html.find("id=\"missing\"").unwrap();
    let known_start = html.find("id=\"known\"").unwrap();
    let reference_start = html.find("id=\"reference\"").unwrap();
    let missing = &html[missing_start..known_start];
    let known = &html[known_start..reference_start];
    let reference = &html[reference_start..];
    assert!(missing.contains("dictionary-tooltip-stack"));
    assert!(!known.contains("dictionary-tooltip-stack"));
    assert_eq!(known.matches("class=\"tooltip-word\"").count(), 1);
    for stack in [missing, reference] {
        let positions = ["klabajra", "klama", "bajra"].map(|word| {
            stack
                .find(&format!("class=\"tooltip-word\">{word}</span>"))
                .unwrap()
        });
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "{stack}"
        );
        assert_eq!(stack.matches("class=\"tooltip-word\"").count(), 3);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_dictionary_link_helpers_keep_component_cards() {
    let direct = dictionary_tooltip_for_word("/tools", "klabajra").unwrap();
    let link =
        cll_dictionary_tooltip_for_link("/tools", CllLinkKind::Dictionary, "klabajra").unwrap();
    let href = cll_dictionary_tooltip_for_href("/tools", "../vlacku/klabajra").unwrap();
    assert_eq!(direct, link);
    assert_eq!(direct, href);
    assert_eq!(direct.components.len(), 2);
}

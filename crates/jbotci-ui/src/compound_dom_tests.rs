use super::*;

#[invariant(true)]
#[invariant(::CompoundDomPage => true)]
#[derive(Clone, PartialEq, Routable)]
enum CompoundDomRoute {
    #[route("/")]
    CompoundDomPage {},
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn CompoundDomApp() -> Element {
    rsx! { Router::<CompoundDomRoute> {} }
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn CompoundDomPage() -> Element {
    let display = consume_context::<Signal<GentufaDisplayState>>();
    let state = *display.read();
    let request = GentufaWebRequest {
        text: "batke zei uidje".to_owned(),
        options: GentufaWebOptions {
            show_compounds: state.show_compounds,
            ..GentufaWebOptions::default()
        },
    };
    let GentufaWebResult::Success(success) = jbotci_web_core::parse_gentufa_for_web(&request)
    else {
        panic!("the DOM fixture is valid syntax");
    };
    let hover = use_signal(ReferenceHoverState::default);
    let tooltip = use_signal(|| None);
    let activity = use_signal(AsyncActivityState::default);
    let export_task = use_signal(|| None);
    let page_find = PageFindContext::new(
        &build_page_find_index("", &[]),
        &PageFindRouteState::default(),
    );
    rsx! {
        { gentufa::render_compounds_checkbox(display, state.show_compounds) }
        { gentufa::render_blocks(&success, true, GentufaScript::Latin, hover, tooltip, activity, export_task, &page_find) }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn compounds_toggle_updates_the_checkbox_and_single_dom_documentation_host() {
    let mut dom = VirtualDom::new(CompoundDomApp);
    let mut display =
        dom.in_runtime(|| Signal::new_in_scope(GentufaDisplayState::default(), ScopeId::ROOT));
    dom.insert_any_root_context(Box::new(display));
    dom.rebuild_in_place();
    for enabled in [true, false, true] {
        if display.read().show_compounds != enabled {
            dom.in_runtime(|| toggle_compounds(&mut display));
            dom.render_immediate_to_vec();
        }
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Compounds"));
        assert_eq!(html.contains("checked"), enabled, "{html}");
        if enabled {
            assert_eq!(
                html.matches("block-label-tooltip dictionary-tooltip-host")
                    .count(),
                1
            );
            assert_eq!(html.matches("class=\"block block-gloss\"").count(), 1);
            assert!(html.contains("data-colspan=\"3\""));
            assert!(html.contains("data-raw-text=\"batke zei uidje\""));
            assert!(html.contains("button (graphical user interface element)"));
        } else {
            assert_eq!(html.matches("class=\"block block-gloss\"").count(), 3);
        }
    }
}

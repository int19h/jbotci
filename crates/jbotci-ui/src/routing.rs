use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum AppRoute {
    Gentufa,
    Settings,
    Cukta,
    Vlacku,
    Gimfihi,
}

pub(super) const TOPBAR_NAV_ROUTES: [AppRoute; 4] = [
    AppRoute::Cukta,
    AppRoute::Vlacku,
    AppRoute::Gentufa,
    AppRoute::Gimfihi,
];

#[invariant(!self.gentufa_text_explicit || matches!(&self.web_route, WebRoute::Gentufa(_)))]
#[invariant(self.settings_query.is_empty() || matches!(&self.web_route, WebRoute::Settings))]
#[invariant(self.hash.as_ref().is_none_or(|hash| !hash.is_empty() && !hash.starts_with('#')))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JbotciRoute {
    pub(super) web_route: WebRoute,
    pub(super) gentufa_text_explicit: bool,
    pub(super) settings_query: String,
    pub(super) hash: Option<String>,
}

impl JbotciRoute {
    #[requires(true)]
    #[ensures(matches!(ret.web_route, WebRoute::Vlacku(_)))]
    pub(super) fn default_vlacku() -> Self {
        new!(JbotciRoute {
            web_route: WebRoute::Vlacku(VlackuWebState::default()),
            gentufa_text_explicit: false,
            settings_query: String::new(),
            hash: None,
        })
    }

    #[requires(true)]
    #[ensures(matches!(ret.web_route, WebRoute::Gentufa(_)))]
    pub(super) fn default_gentufa() -> Self {
        new!(JbotciRoute {
            web_route: WebRoute::Gentufa(GentufaWebState::default()),
            gentufa_text_explicit: false,
            settings_query: String::new(),
            hash: None,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn from_web_route(web_route: WebRoute, gentufa_text_explicit: bool) -> Self {
        new!(JbotciRoute {
            web_route,
            gentufa_text_explicit,
            settings_query: String::new(),
            hash: None,
        })
    }

    #[requires(true)]
    #[ensures(ret == app_route_for_web_route(&self.web_route))]
    pub(super) fn app_route(&self) -> AppRoute {
        app_route_for_web_route(&self.web_route)
    }

    #[requires(true)]
    #[ensures(ret.web_route == self.web_route)]
    pub(super) fn without_hash(&self) -> Self {
        new!(JbotciRoute {
            web_route: self.web_route.clone(),
            gentufa_text_explicit: self.gentufa_text_explicit,
            settings_query: self.settings_query.clone(),
            hash: None,
        })
    }
}

impl Default for JbotciRoute {
    #[requires(true)]
    #[ensures(matches!(ret.web_route, WebRoute::Vlacku(_)))]
    fn default() -> Self {
        Self::default_vlacku()
    }
}

impl fmt::Display for JbotciRoute {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut route = match &self.web_route {
            WebRoute::Settings if !self.settings_query.is_empty() => {
                format!("/settings?{}", self.settings_query)
            }
            _ => web_route_url("", &self.web_route),
        };
        if let Some(hash) = self.hash.as_ref().filter(|hash| !hash.is_empty()) {
            route.push('#');
            route.push_str(hash.trim_start_matches('#'));
        }
        f.write_str(&route)
    }
}

impl FromStr for JbotciRoute {
    type Err = JbotciRouteParseError;

    #[requires(true)]
    #[ensures(true)]
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        jbotci_route_from_dioxus_route(raw).ok_or_else(JbotciRouteParseError::new)
    }
}

impl Routable for JbotciRoute {
    const SITE_MAP: &'static [dioxus::router::SiteMapSegment] = &[];

    #[requires(true)]
    #[ensures(true)]
    fn render(&self, level: usize) -> Element {
        if level == 0 {
            rsx! { AppShell {} }
        } else {
            rsx! {}
        }
    }
}

#[invariant(std::mem::size_of_val(self) == 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct JbotciRouteParseError {
    marker: (),
}

impl JbotciRouteParseError {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        new!(JbotciRouteParseError { marker: () })
    }
}

impl fmt::Display for JbotciRouteParseError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid jbotci route")
    }
}

impl Error for JbotciRouteParseError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct PendingLocalRouteWrites {
    pub(super) routes: Vec<JbotciRoute>,
}

impl PendingLocalRouteWrites {
    #[requires(true)]
    #[ensures(self.routes.iter().any(|pending| pending == &canonical_local_route(route)))]
    pub(super) fn record(&mut self, route: &JbotciRoute) {
        self.routes.push(canonical_local_route(route));
    }

    #[requires(true)]
    #[ensures(ret -> !self.routes.iter().any(|pending| pending == &canonical_local_route(route)))]
    pub(super) fn consume(&mut self, route: &JbotciRoute) -> bool {
        let route = canonical_local_route(route);
        let initial_len = self.routes.len();
        self.routes.retain(|pending| pending != &route);
        self.routes.len() != initial_len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct RouteLocationSyncAction {
    pub(super) app_route: AppRoute,
    pub(super) hydrate_route_bound_state: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GentufaUrlWriteIntent {
    ReplaceCurrent,
    PushParse,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GentufaUrlHistoryAction {
    NoWrite,
    ReplaceCurrent,
    PushParse,
}

#[invariant(*text_explicit || state.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GentufaUrlInputs {
    pub(super) active_route: AppRoute,
    pub(super) current_route: JbotciRoute,
    pub(super) state: GentufaWebState,
    pub(super) text_explicit: bool,
    pub(super) intent: GentufaUrlWriteIntent,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn initial_vlacku_state(route: &JbotciRoute) -> VlackuWebState {
    if let WebRoute::Vlacku(state) = &route.web_route {
        state.clone()
    } else {
        VlackuWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn initial_gimfihi_state(route: &JbotciRoute) -> GimfihiWebState {
    if let WebRoute::Gimfihi(state) = &route.web_route {
        state.clone()
    } else {
        GimfihiWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn initial_cukta_state(route: &JbotciRoute) -> CuktaWebState {
    if let WebRoute::Cukta(state) = &route.web_route {
        state.clone()
    } else {
        CuktaWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn initial_gentufa_state(route: &JbotciRoute) -> GentufaWebState {
    if let WebRoute::Gentufa(state) = &route.web_route {
        state.clone()
    } else {
        GentufaWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn initial_gentufa_text_explicit(route: &JbotciRoute) -> bool {
    route.gentufa_text_explicit
}

#[requires(true)]
#[ensures(ret.is_empty() || ret.starts_with('/'))]
pub(super) fn router_base_path() -> String {
    dioxus::router::router().prefix().unwrap_or_default()
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
pub(super) fn route_href_with_base_path(base_path: &str, route: &JbotciRoute) -> String {
    let route_href = route.to_string();
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        route_href
    } else {
        format!("{prefix}{route_href}")
    }
}

#[requires(base_path.is_empty() || base_path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
pub(super) fn deployment_root_href(base_path: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        "/".to_owned()
    } else {
        format!("{prefix}/")
    }
}

#[requires(base_path.is_empty() || base_path.starts_with('/'))]
#[requires(path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
pub(super) fn static_asset_href_with_base_path(base_path: &str, path: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        path.to_owned()
    } else {
        format!("{prefix}{path}")
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gentufa_state_from_parts(
    text: &str,
    dialect: &str,
    view_mode: GentufaWebViewMode,
    display: GentufaDisplayState,
    text_explicit: bool,
) -> GentufaWebState {
    GentufaWebState {
        text: if text_explicit {
            text.to_owned()
        } else {
            String::new()
        },
        dialect: if dialect.trim().is_empty() {
            None
        } else {
            Some(dialect.to_owned())
        },
        view_mode,
        show_elided: display.show_elided,
        show_glosses: display.show_glosses,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn app_route_for_web_route(route: &WebRoute) -> AppRoute {
    match route {
        WebRoute::Gentufa(_) => AppRoute::Gentufa,
        WebRoute::Cukta(_) => AppRoute::Cukta,
        WebRoute::Vlacku(_) => AppRoute::Vlacku,
        WebRoute::Gimfihi(_) => AppRoute::Gimfihi,
        WebRoute::Settings => AppRoute::Settings,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn strip_base_path_for_client(path: &str, base_path: &str) -> Option<String> {
    let normalized = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    let base = base_path.trim_end_matches('/');
    if base.is_empty() || base == "/" {
        Some(normalized)
    } else if normalized == base {
        Some("/".to_owned())
    } else {
        normalized
            .strip_prefix(&format!("{base}/"))
            .map(|rest| format!("/{rest}"))
    }
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
pub(super) fn is_app_route_path_for_client(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    path.is_empty()
        || path == "/"
        || path == "/gentufa"
        || path.starts_with("/gentufa/")
        || path == "/cukta"
        || path.starts_with("/cukta/")
        || path == "/vlacku"
        || path.starts_with("/vlacku/")
        || is_gimfihi_route_path_for_client(path)
        || path == "/settings"
        || path.starts_with("/settings/")
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
pub(super) fn is_gimfihi_route_path_for_client(path: &str) -> bool {
    matches!(path, "/gimfihi" | "/gimfi'i" | "/gimfi%27i")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn split_href(href: &str) -> (&str, &str, Option<&str>) {
    let (without_hash, hash) = href
        .split_once('#')
        .map(|(before, after)| (before, Some(after)))
        .unwrap_or((href, None));
    let (path, query) = without_hash
        .split_once('?')
        .map(|(path, query)| (path, query))
        .unwrap_or((without_hash, ""));
    (path, query, hash)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn logical_app_path_for_client(path: &str, base_path: &str) -> Option<String> {
    if let Some(logical_path) = strip_base_path_for_client(path, base_path)
        && is_app_route_path_for_client(&logical_path)
    {
        return Some(logical_path);
    }
    let normalized = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    if is_app_route_path_for_client(&normalized) {
        Some(normalized)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn jbotci_route_from_href(base_path: &str, href: &str) -> Option<JbotciRoute> {
    let trimmed = href.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("//")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        return None;
    }
    if !trimmed.starts_with('/') {
        return None;
    }
    let (path, query, hash) = split_href(trimmed);
    let logical_path = logical_app_path_for_client(path, base_path)?;
    let web_route = parse_web_route(&logical_path, query);
    let app_route = app_route_for_web_route(&web_route);
    Some(new!(JbotciRoute {
        gentufa_text_explicit: app_route == AppRoute::Gentufa && query_has_key(query, "text"),
        settings_query: if app_route == AppRoute::Settings {
            query.trim_start_matches('?').to_owned()
        } else {
            String::new()
        },
        hash: hash
            .map(|hash| hash.trim_start_matches('#').to_owned())
            .filter(|hash| !hash.is_empty()),
        web_route,
    }))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn jbotci_route_from_dioxus_route(raw: &str) -> Option<JbotciRoute> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return jbotci_route_from_href("", "/");
    }
    if trimmed.starts_with('/') {
        jbotci_route_from_href("", trimmed)
    } else {
        let href = format!("/{trimmed}");
        jbotci_route_from_href("", &href)
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
pub(super) fn apply_web_route_to_client_state(
    location: &JbotciRoute,
    is_local_route_write: bool,
    mut route: Signal<AppRoute>,
    mut cukta_draft_state: Signal<CuktaWebState>,
    mut cukta_committed_state: Signal<CuktaWebState>,
    mut vlacku_draft_state: Signal<VlackuWebState>,
    mut vlacku_committed_state: Signal<VlackuWebState>,
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    mut gimfihi_committed_state: Signal<GimfihiWebState>,
    mut gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
    mut input_text: Signal<String>,
    mut parsed_text: Signal<String>,
    mut parsed_text_explicit: Signal<bool>,
    mut dialect: Signal<String>,
    mut parsed_dialect: Signal<String>,
    mut view_mode: Signal<GentufaWebViewMode>,
    mut gentufa_display: Signal<GentufaDisplayState>,
) {
    let web_route = &location.web_route;
    let action = route_location_sync_action(location, is_local_route_write);
    set_app_route_if_changed(&mut route, action.app_route);
    if !action.hydrate_route_bound_state {
        return;
    }
    clear_route_bound_input_timers();
    match web_route {
        WebRoute::Gentufa(state) => {
            let input = state.text.clone();
            let parsed = if state.text.is_empty() && !location.gentufa_text_explicit {
                DEFAULT_GENTUFA_TEXT.to_owned()
            } else {
                state.text.clone()
            };
            let dialect_text = state.dialect.clone().unwrap_or_default();
            input_text.set(input);
            parsed_text.set(parsed);
            parsed_text_explicit.set(location.gentufa_text_explicit);
            dialect.set(dialect_text.clone());
            parsed_dialect.set(dialect_text);
            view_mode.set(state.view_mode);
            gentufa_display.set(GentufaDisplayState {
                show_elided: state.show_elided,
                show_glosses: state.show_glosses,
            });
        }
        WebRoute::Cukta(state) => {
            clear_cukta_search_timer();
            cukta_draft_state.set(state.clone());
            cukta_committed_state.set(state.clone());
        }
        WebRoute::Vlacku(state) => {
            clear_vlacku_url_timer();
            clear_vlacku_search_timer();
            vlacku_draft_state.set(state.clone());
            vlacku_committed_state.set(state.clone());
        }
        WebRoute::Gimfihi(state) => {
            gimfihi_source_word_memory.with_mut(|memory| {
                update_gimfihi_source_word_memory(memory, state);
            });
            gimfihi_draft_state.set(state.clone());
            gimfihi_committed_state.set(state.clone());
        }
        WebRoute::Settings => {}
    }
}

#[requires(true)]
#[ensures(ret.app_route == location.app_route())]
#[ensures(ret.hydrate_route_bound_state == !is_local_route_write)]
pub(super) fn route_location_sync_action(
    location: &JbotciRoute,
    is_local_route_write: bool,
) -> RouteLocationSyncAction {
    RouteLocationSyncAction {
        app_route: location.app_route(),
        hydrate_route_bound_state: !is_local_route_write,
    }
}

#[requires(true)]
#[ensures(ret == (current != next))]
pub(super) fn app_route_update_needed(current: AppRoute, next: AppRoute) -> bool {
    current != next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_app_route_if_changed(route: &mut Signal<AppRoute>, next: AppRoute) {
    let current = *route.read();
    if app_route_update_needed(current, next) {
        route.set(next);
    }
}

#[requires(!key.is_empty())]
#[ensures(true)]
pub(super) fn query_has_key(query: &str, key: &str) -> bool {
    query
        .trim_start_matches('?')
        .split('&')
        .filter(|part| !part.is_empty())
        .any(|part| {
            part.split_once('=')
                .map_or(part == key, |(candidate, _)| candidate == key)
        })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn current_hash() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .filter(|hash| !hash.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.is_none())]
pub(super) fn current_hash() -> Option<String> {
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|target| target.contains('#')))]
pub(super) fn cukta_hash_scroll_target(
    path: &str,
    query: &str,
    hash: Option<&str>,
    route: AppRoute,
) -> Option<String> {
    let hash = hash?.trim_start_matches('#');
    if route != AppRoute::Cukta || hash.is_empty() {
        return None;
    }
    Some(format!("{path}{query}#{hash}"))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn current_cukta_pending_scroll(route: &JbotciRoute) -> Option<CuktaPendingScroll> {
    cukta_hash_scroll_target(
        &current_path(),
        &current_query(),
        current_hash().as_deref(),
        route.app_route(),
    )
    .map(cukta_anchor_pending_scroll)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_anchor_pending_scroll(target: String) -> CuktaPendingScroll {
    CuktaPendingScroll {
        mode: CuktaPendingScrollMode::Anchor,
        target,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_stored_pending_scroll(target: String) -> CuktaPendingScroll {
    CuktaPendingScroll {
        mode: CuktaPendingScrollMode::Stored,
        target,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_top_pending_scroll() -> CuktaPendingScroll {
    CuktaPendingScroll {
        mode: CuktaPendingScrollMode::Top,
        target: String::new(),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_pending_scroll_for_navigation(
    route: AppRoute,
    target: &str,
    has_hash: bool,
    restore_stored: bool,
) -> Option<CuktaPendingScroll> {
    if route != AppRoute::Cukta {
        return None;
    }
    if has_hash {
        Some(cukta_anchor_pending_scroll(target.to_owned()))
    } else if restore_stored {
        Some(cukta_stored_pending_scroll(target.to_owned()))
    } else {
        Some(cukta_top_pending_scroll())
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_pending_scroll_for_route_change(
    base_path: &str,
    route: &JbotciRoute,
) -> Option<CuktaPendingScroll> {
    if route.app_route() != AppRoute::Cukta {
        return None;
    }
    let target = route_href_with_base_path(base_path, route);
    Some(cukta_stored_pending_scroll(target))
}

#[requires(route.app_route() == AppRoute::Cukta)]
#[ensures(matches!(ret.mode, CuktaPendingScrollMode::Anchor) == route.hash.is_some())]
pub(super) fn cukta_pending_scroll_for_route_link(
    base_path: &str,
    route: &JbotciRoute,
) -> CuktaPendingScroll {
    if route.hash.is_some() {
        cukta_anchor_pending_scroll(route_href_with_base_path(base_path, route))
    } else {
        cukta_top_pending_scroll()
    }
}

#[requires(true)]
#[ensures(route.app_route() == AppRoute::Cukta -> ret.is_some())]
#[ensures(route.app_route() != AppRoute::Cukta -> ret.is_none())]
pub(super) fn cukta_pending_scroll_for_explicit_route_link(
    base_path: &str,
    route: &JbotciRoute,
) -> Option<CuktaPendingScroll> {
    if route.app_route() == AppRoute::Cukta {
        Some(cukta_pending_scroll_for_route_link(base_path, route))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_route_with_cukta_scroll_intent(
    mut pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    pending_scroll: Option<CuktaPendingScroll>,
    route: JbotciRoute,
) {
    if let Some(scroll) = pending_scroll {
        pending_cukta_scroll.set(Some(scroll));
    }
    let _ = navigator().push(route);
}

#[requires(true)]
#[ensures(!ret || page.state.as_ref().is_some_and(|page_state| page_state == state))]
#[ensures(!ret || !page.loading)]
#[ensures(!ret || page.error.is_none())]
pub(super) fn cukta_page_ready_for_scroll(
    page: &CuktaAsyncPageState,
    state: &CuktaWebState,
) -> bool {
    page.state
        .as_ref()
        .is_some_and(|page_state| page_state == state)
        && !page.loading
        && page.error.is_none()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn apply_cukta_pending_scroll(scroll: CuktaPendingScroll) {
    match scroll.mode {
        CuktaPendingScrollMode::Anchor => scroll_to_cukta_href(&scroll.target),
        CuktaPendingScrollMode::Stored => restore_scroll_for_url(&scroll.target),
        CuktaPendingScrollMode::Top => scroll_to_top(),
    }
}

#[requires(true)]
#[ensures(ret.starts_with("jbotci.scroll."))]
pub(super) fn scroll_storage_key(path_query_or_url: &str) -> String {
    let (path, query, _) = split_href(path_query_or_url);
    if query.is_empty() {
        format!("jbotci.scroll.{path}")
    } else {
        format!("jbotci.scroll.{path}?{query}")
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!selector.is_empty())]
#[ensures(true)]
pub(super) fn scroll_container_by_selector(selector: &str) -> Option<web_sys::HtmlElement> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(selector).ok().flatten())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_container_is_scrollable(element: &web_sys::HtmlElement) -> bool {
    element.scroll_height() > element.client_height()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_scroll_container() -> Option<web_sys::HtmlElement> {
    scroll_container_by_selector("[data-cukta-scroll='main']")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn active_scroll_container() -> Option<web_sys::HtmlElement> {
    cukta_scroll_container()
        .filter(scroll_container_is_scrollable)
        .or_else(|| {
            scroll_container_by_selector("[data-app-scroll='main']")
                .filter(scroll_container_is_scrollable)
        })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn element_scroll_margin_top(element: &web_sys::Element) -> f64 {
    web_sys::window()
        .and_then(|window| window.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("scroll-margin-top").ok())
        .and_then(|value| value.trim().strip_suffix("px")?.parse::<f64>().ok())
        .unwrap_or(0.0)
        .max(0.0)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_container_to_y(y: i32) {
    if let Some(element) = active_scroll_container() {
        element.set_scroll_top(y.max(0));
    } else if let Some(window) = web_sys::window() {
        window.scroll_to_with_x_and_y(0.0, f64::from(y.max(0)));
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_scroll_container_to_y(y: i32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || scroll_container_to_y(y));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        30,
    );
    closure.forget();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_scroll_container_to_y(_y: i32) {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_to_cukta_anchor_element(element: &web_sys::Element) {
    let Some(container) = cukta_scroll_container().or_else(active_scroll_container) else {
        element.scroll_into_view();
        return;
    };
    let container_rect = container.get_bounding_client_rect();
    let element_rect = element.get_bounding_client_rect();
    let next_scroll_top = f64::from(container.scroll_top()) + element_rect.top()
        - container_rect.top()
        - element_scroll_margin_top(element);
    container.set_scroll_top(next_scroll_top.round().max(0.0) as i32);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn save_current_scroll_position() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let key = scroll_storage_key(&format!(
        "{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    ));
    let y = current_scroll_y();
    session_storage_set(&key, &y.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn save_current_scroll_position() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn restore_scroll_for_current_url() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    restore_scroll_for_url(&format!(
        "{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn restore_scroll_for_current_url() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0)]
pub(super) fn current_scroll_y() -> i32 {
    active_scroll_container()
        .map(|element| element.scroll_top().max(0))
        .unwrap_or_else(|| {
            web_sys::window()
                .and_then(|window| window.scroll_y().ok())
                .unwrap_or(0.0)
                .round()
                .max(0.0) as i32
        })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret == 0)]
pub(super) fn current_scroll_y() -> i32 {
    0
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_to_top() {
    schedule_scroll_container_to_y(0);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_to_top() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn restore_scroll_for_url(url: &str) {
    let key = scroll_storage_key(url);
    let Some(raw) = session_storage_get(&key) else {
        scroll_container_to_y(0);
        return;
    };
    let Ok(y) = raw.parse::<i32>() else {
        return;
    };
    schedule_scroll_container_to_y(y);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn restore_scroll_for_url(url: &str) {
    let _ = url;
}

#[requires(true)]
#[ensures(ret.mode == state.mode)]
#[ensures(ret.query == state.query)]
#[ensures(ret.word_types == state.word_types)]
#[ensures(ret.count >= 1 && ret.count <= VLACKU_WEB_MAX_COUNT)]
pub(super) fn vlacku_load_more_state(state: &VlackuWebState) -> VlackuWebState {
    let mut next = state.clone();
    next.count = next.count.saturating_mul(2).clamp(1, VLACKU_WEB_MAX_COUNT);
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_vlacku_state_immediate(
    draft_state: &mut Signal<VlackuWebState>,
    committed_state: &mut Signal<VlackuWebState>,
    state: VlackuWebState,
) {
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    draft_state.set(state.clone());
    committed_state.set(state);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_vlacku_search_commit(
    mut committed_state: Signal<VlackuWebState>,
    state: VlackuWebState,
) {
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    if let Some(handle) = platform::schedule_timeout_once(VLACKU_SEARCH_DEBOUNCE_MS, move || {
        committed_state.set(state);
    }) {
        VLACKU_SEARCH_TIMER.with(|timer| timer.set(Some(handle)));
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_cukta_search_commit(
    mut committed_state: Signal<CuktaWebState>,
    state: CuktaWebState,
) {
    clear_cukta_search_timer();
    if let Some(handle) = platform::schedule_timeout_once(CUKTA_SEARCH_DEBOUNCE_MS, move || {
        committed_state.set(state);
    }) {
        CUKTA_SEARCH_TIMER.with(|timer| timer.set(Some(handle)));
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_vlacku_search_timer() {
    VLACKU_SEARCH_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            platform::clear_timeout(handle);
        }
    });
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_cukta_search_timer() {
    CUKTA_SEARCH_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            platform::clear_timeout(handle);
        }
    });
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_vlacku_url_timer() {
    VLACKU_URL_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            platform::clear_timeout(handle);
        }
    });
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_route_bound_input_timers() {
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    clear_cukta_search_timer();
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_vlacku_url_push(
    history: Rc<dyn History>,
    pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &VlackuWebState,
    restore_scroll_y: Option<i32>,
) {
    let target = JbotciRoute::from_web_route(WebRoute::Vlacku(state.clone()), false);
    if current.without_hash() == target {
        return;
    }
    schedule_route_push(
        history,
        pending_writes,
        target,
        VLACKU_URL_DEBOUNCE_MS,
        restore_scroll_y,
    );
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_route_push(
    history: Rc<dyn History>,
    pending_writes: Signal<PendingLocalRouteWrites>,
    target: JbotciRoute,
    delay_ms: i32,
    restore_scroll_y: Option<i32>,
) {
    clear_vlacku_url_timer();
    if let Some(handle) = platform::schedule_timeout_once(delay_ms, move || {
        let mut pending_writes = pending_writes;
        pending_writes.with_mut(|pending| pending.record(&target));
        history.push(route_path_for_route(&target));
        if let Some(y) = restore_scroll_y {
            schedule_scroll_container_to_y(y);
        }
    }) {
        VLACKU_URL_TIMER.with(|timer| timer.set(Some(handle)));
    }
}

#[requires(true)]
#[ensures(ret.app_route() == AppRoute::Gentufa)]
#[ensures(ret.gentufa_text_explicit == text_explicit)]
pub(super) fn gentufa_route_for_committed_state(
    state: &GentufaWebState,
    text_explicit: bool,
) -> JbotciRoute {
    JbotciRoute::from_web_route(WebRoute::Gentufa(state.clone()), text_explicit)
}

#[requires(true)]
#[ensures(ret == (active_route == AppRoute::Gentufa && current_route.app_route() == AppRoute::Gentufa))]
pub(super) fn gentufa_url_sync_allowed(
    active_route: AppRoute,
    current_route: &JbotciRoute,
) -> bool {
    active_route == AppRoute::Gentufa && current_route.app_route() == AppRoute::Gentufa
}

#[requires(true)]
#[ensures((current.without_hash() == *target) == (ret == GentufaUrlHistoryAction::NoWrite))]
pub(super) fn gentufa_url_history_action(
    current: &JbotciRoute,
    target: &JbotciRoute,
    intent: GentufaUrlWriteIntent,
) -> GentufaUrlHistoryAction {
    if current.without_hash() == *target {
        GentufaUrlHistoryAction::NoWrite
    } else {
        match intent {
            GentufaUrlWriteIntent::ReplaceCurrent => GentufaUrlHistoryAction::ReplaceCurrent,
            GentufaUrlWriteIntent::PushParse => GentufaUrlHistoryAction::PushParse,
        }
    }
}

#[requires(true)]
#[ensures(action == GentufaUrlHistoryAction::NoWrite -> ret == GentufaUrlWriteIntent::ReplaceCurrent)]
#[ensures(action != GentufaUrlHistoryAction::NoWrite -> ret == intent)]
pub(super) fn gentufa_url_intent_after_sync_action(
    intent: GentufaUrlWriteIntent,
    action: GentufaUrlHistoryAction,
) -> GentufaUrlWriteIntent {
    match action {
        GentufaUrlHistoryAction::NoWrite => GentufaUrlWriteIntent::ReplaceCurrent,
        GentufaUrlHistoryAction::ReplaceCurrent | GentufaUrlHistoryAction::PushParse => intent,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_gentufa_url_write_intent_if_changed(
    intent: &mut Signal<GentufaUrlWriteIntent>,
    current: GentufaUrlWriteIntent,
    next: GentufaUrlWriteIntent,
) {
    if current != next {
        intent.set(next);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn sync_gentufa_committed_url(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &GentufaWebState,
    text_explicit: bool,
    write_intent: GentufaUrlWriteIntent,
    mut intent_signal: Signal<GentufaUrlWriteIntent>,
) {
    let target = gentufa_route_for_committed_state(state, text_explicit);
    let action = gentufa_url_history_action(current, &target, write_intent);
    match action {
        GentufaUrlHistoryAction::NoWrite => {}
        GentufaUrlHistoryAction::ReplaceCurrent => {
            pending_writes.with_mut(|pending| pending.record(&target));
            history.replace(route_path_for_route(&target));
        }
        GentufaUrlHistoryAction::PushParse => {
            pending_writes.with_mut(|pending| pending.record(&target));
            history.push(route_path_for_route(&target));
        }
    }
    let next_intent = gentufa_url_intent_after_sync_action(write_intent, action);
    set_gentufa_url_write_intent_if_changed(&mut intent_signal, write_intent, next_intent);
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
pub(super) fn route_path_for_route(route: &JbotciRoute) -> String {
    route.to_string()
}

#[requires(true)]
#[ensures(route_path_for_route(&ret).starts_with('/'))]
pub(super) fn canonical_local_route(route: &JbotciRoute) -> JbotciRoute {
    jbotci_route_from_dioxus_route(&route_path_for_route(route)).unwrap_or_else(|| route.clone())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_cukta_url(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &CuktaWebState,
) {
    let target = JbotciRoute::from_web_route(WebRoute::Cukta(state.clone()), false);
    if current.without_hash() == target {
        return;
    }
    pending_writes.with_mut(|pending| pending.record(&target));
    history.push(route_path_for_route(&target));
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_gimfihi_url(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &GimfihiWebState,
) {
    let target = JbotciRoute::from_web_route(WebRoute::Gimfihi(state.clone()), false);
    if current.without_hash() == target {
        return;
    }
    pending_writes.with_mut(|pending| pending.record(&target));
    history.push(route_path_for_route(&target));
}

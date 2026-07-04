#[invariant(true)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PageFindState {
    cukta: PageFindRouteState,
    vlacku: PageFindRouteState,
    gimfihi: PageFindRouteState,
    gentufa: PageFindRouteState,
    settings: PageFindRouteState,
}

impl PageFindState {
    #[requires(true)]
    #[ensures(true)]
    fn route_state(&self, route: AppRoute) -> &PageFindRouteState {
        match route {
            AppRoute::Cukta => &self.cukta,
            AppRoute::Vlacku => &self.vlacku,
            AppRoute::Gimfihi => &self.gimfihi,
            AppRoute::Gentufa => &self.gentufa,
            AppRoute::Settings => &self.settings,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn route_state_mut(&mut self, route: AppRoute) -> &mut PageFindRouteState {
        match route {
            AppRoute::Cukta => &mut self.cukta,
            AppRoute::Vlacku => &mut self.vlacku,
            AppRoute::Gimfihi => &mut self.gimfihi,
            AppRoute::Gentufa => &mut self.gentufa,
            AppRoute::Settings => &mut self.settings,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn set_page_find_query(
    state: &mut PageFindState,
    route: AppRoute,
    query: String,
    update: PageFindRouteQueryUpdate,
) {
    let route_state = state.route_state_mut(route);
    match update {
        PageFindRouteQueryUpdate::Replace => {
            if route_state.query != query {
                *route_state = route_state.clone().with_data(data! {
                    query: query,
                    active_index: None,
                    result_signature: 0,
                });
            }
        }
        PageFindRouteQueryUpdate::Clear => {
            if !route_state.query.is_empty() || route_state.active_index.is_some() {
                *route_state = route_state.clone().with_data(data! {
                    query: String::new(),
                    active_index: None,
                    result_signature: 0,
                });
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn update_page_find_active(
    state: &mut PageFindState,
    route: AppRoute,
    direction: PageFindDirection,
    match_count: usize,
) {
    if match_count == 0 {
        let route_state = state.route_state_mut(route);
        reset_page_find_active(route_state);
        return;
    }
    let route_state = state.route_state_mut(route);
    let next = match (route_state.active_index, direction) {
        (Some(index), PageFindDirection::Next) => (index + 1) % match_count,
        (Some(0), PageFindDirection::Previous) => match_count - 1,
        (Some(index), PageFindDirection::Previous) => index - 1,
        (None, PageFindDirection::Next) => 0,
        (None, PageFindDirection::Previous) => match_count - 1,
    };
    *route_state = route_state.clone().with_data(data! {
        active_index: Some(next),
        scroll_request: route_state.scroll_request.wrapping_add(1),
    });
}

#[requires(true)]
#[ensures(true)]
fn sync_page_find_result_signature(
    state: &mut PageFindState,
    route: AppRoute,
    signature: u64,
    match_count: usize,
) {
    let route_state = state.route_state_mut(route);
    if route_state.result_signature != signature {
        *route_state = route_state.clone().with_data(data! {
            result_signature: signature,
            active_index: None,
        });
        return;
    }
    if route_state
        .active_index
        .is_some_and(|active_index| active_index >= match_count)
    {
        reset_page_find_active(route_state);
    }
}

#[requires(true)]
#[ensures(route_state.active_index.is_none())]
fn reset_page_find_active(route_state: &mut PageFindRouteState) {
    if route_state.active_index.is_some() {
        *route_state = route_state.clone().with_data(data! { active_index: None });
    }
}

#[invariant(self.active_index.is_none() || !self.query.is_empty())]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PageFindRouteState {
    query: String,
    active_index: Option<usize>,
    result_signature: u64,
    scroll_request: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
struct PageFindTextKey {
    content_hash: u64,
    occurrence: usize,
}

#[invariant(byte_start <= byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PageFindTextRange {
    byte_start: usize,
    byte_end: usize,
}

#[invariant(self.range.byte_start < self.range.byte_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindMatch {
    key: PageFindTextKey,
    range: PageFindTextRange,
    index: usize,
}

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindTextEntry {
    key: PageFindTextKey,
    text: String,
}

#[invariant(!self.query.is_empty() || self.matches.is_empty())]
#[invariant(self.matches.iter().enumerate().all(|(expected, page_match)| page_match.index == expected))]
#[invariant(self.matches_by_key.values().map(Vec::len).sum::<usize>() == self.matches.len())]
#[invariant(self.matches_by_key.values().flatten().all(|page_match| self.matches.get(page_match.index).is_some_and(|indexed| indexed == page_match)))]
#[invariant(self.matches.iter().all(|page_match| self.matches_by_key.get(&page_match.key).is_some_and(|key_matches| key_matches.iter().any(|mapped| mapped == page_match))))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindIndex {
    query: String,
    matches: Vec<PageFindMatch>,
    matches_by_key: BTreeMap<PageFindTextKey, Vec<PageFindMatch>>,
    entry_keys: Vec<PageFindTextKey>,
    signature: u64,
}

#[invariant(self.active_index.is_none_or(|index| index < self.match_count))]
#[derive(Debug, Clone)]
struct PageFindContext {
    query: String,
    active_index: Option<usize>,
    match_count: usize,
    matches_by_key: Rc<BTreeMap<PageFindTextKey, Vec<PageFindMatch>>>,
    entry_keys: Rc<Vec<PageFindTextKey>>,
    next_entry_key_index: Rc<Cell<usize>>,
}

impl PartialEq for PageFindContext {
    #[requires(true)]
    #[ensures(true)]
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
            && self.active_index == other.active_index
            && self.match_count == other.match_count
            && self.matches_by_key == other.matches_by_key
    }
}

impl Eq for PageFindContext {}

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindRenderPiece {
    text: String,
    match_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum PageFindDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum PageFindRouteQueryUpdate {
    Replace,
    Clear,
}

impl PageFindContext {
    #[requires(true)]
    #[ensures(ret.match_count == index.matches.len())]
    fn new(index: &PageFindIndex, route_state: &PageFindRouteState) -> Self {
        let active_index = if route_state.result_signature == index.signature {
            route_state
                .active_index
                .filter(|active_index| *active_index < index.matches.len())
        } else {
            None
        };
        new!(PageFindContext {
            query: index.query.clone(),
            active_index,
            match_count: index.matches.len(),
            matches_by_key: Rc::new(index.matches_by_key.clone()),
            entry_keys: Rc::new(index.entry_keys.clone()),
            next_entry_key_index: Rc::new(Cell::new(0)),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_text_key(&self) -> PageFindTextKey {
        let index = self.next_entry_key_index.get();
        self.next_entry_key_index.set(index.saturating_add(1));
        self.entry_keys
            .get(index)
            .copied()
            .unwrap_or(PageFindTextKey {
                content_hash: 0,
                occurrence: usize::MAX,
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn matches_for_key(&self, key: PageFindTextKey) -> &[PageFindMatch] {
        self.matches_by_key
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[requires(true)]
#[ensures(ret.query == query)]
fn build_page_find_index(query: &str, entries: &[PageFindTextEntry]) -> PageFindIndex {
    let signature = page_find_result_signature(query, entries);
    let mut matches = Vec::<PageFindMatch>::new();
    let mut matches_by_key = BTreeMap::<PageFindTextKey, Vec<PageFindMatch>>::new();
    let entry_keys = entries.iter().map(|entry| entry.key).collect::<Vec<_>>();
    if query.is_empty() {
        return new!(PageFindIndex {
            query: query.to_owned(),
            matches,
            matches_by_key,
            entry_keys,
            signature,
        });
    }

    for entry in entries {
        let key = entry.key;
        for range in page_find_match_ranges(&entry.text, query) {
            let index = matches.len();
            let page_match = new!(PageFindMatch { key, range, index });
            matches_by_key
                .entry(key)
                .or_default()
                .push(page_match.clone());
            matches.push(page_match);
        }
    }

    new!(PageFindIndex {
        query: query.to_owned(),
        matches,
        matches_by_key,
        entry_keys,
        signature,
    })
}

#[requires(true)]
#[ensures(true)]
fn page_find_result_signature(query: &str, entries: &[PageFindTextEntry]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    entries.len().hash(&mut hasher);
    for entry in entries {
        entry.key.hash(&mut hasher);
        entry.text.hash(&mut hasher);
    }
    hasher.finish()
}

#[requires(true)]
#[ensures(ret.iter().all(|range| range.byte_start < range.byte_end))]
fn page_find_match_ranges(text: &str, query: &str) -> Vec<PageFindTextRange> {
    if text.is_empty() || query.is_empty() {
        return Vec::new();
    }
    let normalized_text = normalized_page_find_text(text);
    let normalized_query = lowercase_page_find_text(query);
    if normalized_text.text.is_empty() || normalized_query.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut search_start = 0;
    while search_start <= normalized_text.text.len() {
        let Some(relative_start) = normalized_text.text[search_start..].find(&normalized_query)
        else {
            break;
        };
        let normalized_start = search_start + relative_start;
        let normalized_end = normalized_start + normalized_query.len();
        if let Some(range) = original_range_for_normalized_match(
            &normalized_text.spans,
            normalized_start,
            normalized_end,
        ) {
            ranges.push(range);
        }
        search_start = normalized_end;
    }
    ranges
}

#[invariant(self.spans.iter().all(|span| span.normalized_end <= self.text.len()))]
#[invariant(self.spans.windows(2).all(|pair| pair[0].normalized_end <= pair[1].normalized_start))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedPageFindText {
    text: String,
    spans: Vec<NormalizedPageFindCharSpan>,
}

#[invariant(normalized_start <= normalized_end)]
#[invariant(original_start <= original_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NormalizedPageFindCharSpan {
    normalized_start: usize,
    normalized_end: usize,
    original_start: usize,
    original_end: usize,
}

#[requires(true)]
#[ensures(true)]
fn normalized_page_find_text(text: &str) -> NormalizedPageFindText {
    let mut normalized = String::new();
    let mut spans = Vec::new();
    for (original_start, character) in text.char_indices() {
        let original_end = original_start + character.len_utf8();
        for lower in character.to_lowercase() {
            let normalized_start = normalized.len();
            normalized.push(lower);
            let normalized_end = normalized.len();
            spans.push(new!(NormalizedPageFindCharSpan {
                normalized_start,
                normalized_end,
                original_start,
                original_end,
            }));
        }
    }
    new!(NormalizedPageFindText {
        text: normalized,
        spans,
    })
}

#[requires(true)]
#[ensures(true)]
fn lowercase_page_find_text(text: &str) -> String {
    let mut normalized = String::new();
    for character in text.chars() {
        normalized.extend(character.to_lowercase());
    }
    normalized
}

#[requires(normalized_start < normalized_end)]
#[ensures(ret.is_none_or(|range| range.byte_start < range.byte_end))]
fn original_range_for_normalized_match(
    spans: &[NormalizedPageFindCharSpan],
    normalized_start: usize,
    normalized_end: usize,
) -> Option<PageFindTextRange> {
    let first = spans
        .iter()
        .find(|span| span.normalized_end > normalized_start)?;
    let last = spans
        .iter()
        .rev()
        .find(|span| span.normalized_start < normalized_end)?;
    Some(new!(PageFindTextRange {
        byte_start: first.original_start,
        byte_end: last.original_end,
    }))
}

#[requires(true)]
#[ensures(true)]
fn push_page_find_entry(entries: &mut Vec<PageFindTextEntry>, text: impl Into<String>) {
    let text = text.into();
    if !text.is_empty() {
        let key = page_find_text_key(entries, &text);
        entries.push(new!(PageFindTextEntry { key, text }));
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn page_find_text_key(entries: &[PageFindTextEntry], text: &str) -> PageFindTextKey {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    let content_hash = hasher.finish();
    let occurrence = entries
        .iter()
        .filter(|entry| entry.key.content_hash == content_hash)
        .count();
    PageFindTextKey {
        content_hash,
        occurrence,
    }
}

#[requires(true)]
#[ensures(true)]
fn page_find_render_pieces(text: &str, matches: &[PageFindMatch]) -> Vec<PageFindRenderPiece> {
    if text.is_empty() {
        return Vec::new();
    }
    if matches.is_empty() {
        return vec![new!(PageFindRenderPiece {
            text: text.to_owned(),
            match_index: None,
        })];
    }
    let mut pieces = Vec::new();
    let mut cursor = 0;
    for page_match in matches {
        if page_match.range.byte_start > cursor {
            pieces.push(new!(PageFindRenderPiece {
                text: text[cursor..page_match.range.byte_start].to_owned(),
                match_index: None,
            }));
        }
        pieces.push(new!(PageFindRenderPiece {
            text: text[page_match.range.byte_start..page_match.range.byte_end].to_owned(),
            match_index: Some(page_match.index),
        }));
        cursor = page_match.range.byte_end;
    }
    if cursor < text.len() {
        pieces.push(new!(PageFindRenderPiece {
            text: text[cursor..].to_owned(),
            match_index: None,
        }));
    }
    pieces.retain(|piece| !piece.text.is_empty());
    pieces
}

#[requires(true)]
#[ensures(true)]
fn render_page_find_text(page_find: &PageFindContext, text: &str) -> Element {
    if text.is_empty() {
        return rsx! {};
    }
    let key = page_find.next_text_key();
    let matches = page_find.matches_for_key(key);
    if matches.is_empty() {
        return rsx! { "{text}" };
    }
    let pieces = page_find_render_pieces(text, matches);
    rsx! {
        for piece in pieces.iter() {
            if let Some(match_index) = piece.match_index {
                {
                    let class_name = page_find_mark_class(page_find.active_index == Some(match_index));
                    rsx! {
                        mark {
                            class: "{class_name}",
                            "data-page-find-match-index": "{match_index}",
                            aria_current: if page_find.active_index == Some(match_index) { "true" } else { "false" },
                            "{piece.text}"
                        }
                    }
                }
            } else {
                "{piece.text}"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_optional_page_find_text(page_find: Option<&PageFindContext>, text: &str) -> Element {
    if let Some(page_find) = page_find {
        render_page_find_text(page_find, text)
    } else {
        rsx! { "{text}" }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn page_find_mark_class(active: bool) -> &'static str {
    if active {
        "page-find-hit is-active"
    } else {
        "page-find-hit"
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn page_find_entries_for_route(
    route: AppRoute,
    cukta_page: &CuktaAsyncPageState,
    vlacku_committed_state: &VlackuWebState,
    vlacku_result_state: &VlackuAsyncResultState,
    gimfihi_committed_state: &GimfihiWebState,
    gimfihi_result_state: &GimfihiAsyncResultState,
    gentufa_result: &GentufaWebResult,
    gentufa_request: Option<&GentufaWebRequest>,
    gentufa_view_mode: GentufaWebViewMode,
    gentufa_display: GentufaDisplayState,
    current_settings: UserSettings,
    dialect_settings: &DialectSettings,
    selected_dialect: &str,
    embedding_settings: &EmbeddingSettingsState,
    script: GentufaScript,
) -> Vec<PageFindTextEntry> {
    let mut entries = Vec::new();
    match route {
        AppRoute::Cukta => collect_cukta_page_find_entries(&mut entries, &cukta_page.page, script),
        AppRoute::Vlacku => {
            let result =
                visible_vlacku_result_for_find(vlacku_committed_state, vlacku_result_state);
            collect_vlacku_page_find_entries(&mut entries, &result, script);
        }
        AppRoute::Gimfihi => {
            let result =
                visible_gimfihi_result_for_find(gimfihi_committed_state, gimfihi_result_state);
            collect_gimfihi_page_find_entries(&mut entries, &result);
        }
        AppRoute::Gentufa => collect_gentufa_page_find_entries(
            &mut entries,
            gentufa_result,
            gentufa_request,
            gentufa_view_mode,
            gentufa_display,
            script,
        ),
        AppRoute::Settings => collect_settings_page_find_entries(
            &mut entries,
            current_settings,
            dialect_settings,
            selected_dialect,
            embedding_settings,
        ),
    }
    entries
}

#[requires(true)]
#[ensures(true)]
fn visible_vlacku_result_for_find(
    committed_state: &VlackuWebState,
    result_state: &VlackuAsyncResultState,
) -> VlackuWebResult {
    if result_state.state.as_ref() == Some(committed_state) {
        result_state.result.clone()
    } else {
        vlacku_loading_result(committed_state, "Loading dictionary results.")
    }
}

#[requires(true)]
#[ensures(true)]
fn visible_gimfihi_result_for_find(
    committed_state: &GimfihiWebState,
    result_state: &GimfihiAsyncResultState,
) -> GimfihiWebResult {
    if result_state.state.as_ref() == Some(committed_state) {
        result_state.result.clone()
    } else {
        gimfihi_empty_result(committed_state)
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cukta_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    page: &CuktaPageData,
    script: GentufaScript,
) {
    let site = embedded_cll_site().ok();
    match &page.page_kind {
        CuktaPageKind::Section {
            section_heading,
            chapter_prelude_blocks,
            blocks,
            previous_section,
            next_section,
            ..
        } => {
            push_page_find_entry(entries, section_heading.clone());
            for block in chapter_prelude_blocks {
                collect_cll_block_page_find_entries(entries, site, block, script);
            }
            for block in blocks {
                collect_cll_block_page_find_entries(entries, site, block, script);
            }
            if let Some(previous) = previous_section {
                push_page_find_entry(entries, previous.label.clone());
            }
            if let Some(next) = next_section {
                push_page_find_entry(entries, next.label.clone());
            }
        }
        CuktaPageKind::Index {
            entries: index_entries,
        } => {
            push_page_find_entry(entries, "Index");
            for entry in index_entries {
                push_page_find_entry(entries, entry.key.clone());
                for reference in &entry.references {
                    push_page_find_entry(entries, reference.label.clone());
                }
            }
        }
        CuktaPageKind::Search {
            results,
            message,
            has_more,
            ..
        } => {
            if let Some(message) = message {
                push_page_find_entry(entries, semantic_search_message_visible_text(message));
            }
            for card in results {
                collect_cukta_search_card_page_find_entries(entries, card);
            }
            if *has_more {
                push_page_find_entry(entries, "Load more");
            }
        }
        CuktaPageKind::Error { message } => push_page_find_entry(entries, message.clone()),
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cukta_search_card_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    card: &CuktaSearchResultCard,
) {
    push_page_find_entry(entries, format!("{} · {}", card.kind, card.section_label));
    push_page_find_entry(entries, format!("{}. {}", card.rank, card.label));
    if let Some(similarity) = &card.similarity_label {
        push_page_find_entry(entries, similarity.clone());
    }
    push_page_find_entry(entries, card.preview.clone());
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_block_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    site: Option<&jbotci_cll::CllSite>,
    block: &CllBlock,
    script: GentufaScript,
) {
    match block {
        CllBlock::Paragraph { inlines, text, .. } => {
            if inlines.is_empty() {
                push_page_find_entry(entries, text.clone());
            } else {
                collect_cll_inlines_page_find_entries(entries, inlines, script, false);
            }
        }
        CllBlock::List { items, .. } => {
            for item in items {
                for child in item {
                    collect_cll_block_page_find_entries(entries, site, child, script);
                }
            }
        }
        CllBlock::Example { example_id } => {
            if let Some(example) =
                site.and_then(|site| jbotci_cll::cll_lookup_example(site, example_id))
            {
                push_page_find_entry(entries, example.label.clone());
                if example.blocks.is_empty() {
                    for line in &example.lines {
                        push_page_find_entry(
                            entries,
                            cll_display_text_for_kind(script, line.kind.as_str(), &line.text),
                        );
                    }
                } else {
                    for child in &example.blocks {
                        collect_cll_block_page_find_entries(entries, site, child, script);
                    }
                }
            }
        }
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => {
            if let Some(caption) = caption {
                collect_cll_inlines_page_find_entries(entries, caption, script, false);
            }
            for row in header_rows.iter().chain(body_rows.iter()) {
                for cell in row {
                    collect_cll_table_cell_page_find_entries(entries, site, cell, script);
                }
            }
        }
        CllBlock::SimpleListTable { rows, .. } => {
            for row in rows {
                for cell in row {
                    if let Some(inlines) = cell {
                        collect_cll_inlines_page_find_entries(entries, inlines, script, false);
                    }
                }
            }
        }
        CllBlock::VariableList { entries: items, .. } => {
            for entry in items {
                collect_cll_inlines_page_find_entries(entries, &entry.term, script, false);
                for child in &entry.blocks {
                    collect_cll_block_page_find_entries(entries, site, child, script);
                }
            }
        }
        CllBlock::Media { title, .. } => {
            if let Some(title) = title {
                collect_cll_inlines_page_find_entries(entries, title, script, false);
            }
        }
        CllBlock::Rule { term, body, .. } => {
            push_page_find_entry(entries, term.clone());
            for child in body {
                collect_cll_block_page_find_entries(entries, site, child, script);
            }
        }
        CllBlock::Code { text, .. } => push_page_find_entry(entries, text.clone()),
        CllBlock::DisplayMath { .. } => {}
        CllBlock::Heading { inlines, .. } => {
            collect_cll_inlines_page_find_entries(entries, inlines, script, false);
        }
        CllBlock::BlockQuote { blocks, .. } => {
            for child in blocks {
                collect_cll_block_page_find_entries(entries, site, child, script);
            }
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            collect_cll_inlines_page_find_entries(entries, body, script, false);
        }
        CllBlock::InterlinearGloss {
            rows,
            natlang,
            comments,
            ..
        } => {
            for row in rows {
                let row_context = row.kind.is_lojban();
                for cell in &row.cells {
                    collect_cll_inlines_page_find_entries(entries, cell, script, row_context);
                }
            }
            for comment in comments {
                collect_cll_inlines_page_find_entries(entries, comment, script, false);
            }
            for line in natlang {
                collect_cll_inlines_page_find_entries(entries, line, script, false);
            }
        }
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => {
            for title in titles {
                collect_cll_inlines_page_find_entries(entries, title, script, false);
            }
            for header in headers {
                collect_cll_inlines_page_find_entries(entries, header, script, false);
            }
            for row in rows {
                for cell in row {
                    collect_cll_inlines_page_find_entries(entries, cell, script, false);
                }
            }
        }
        CllBlock::Lojbanization { lines, .. } => {
            for line in lines {
                push_page_find_entry(entries, line.kind.as_str());
                let line_context = line.kind.is_lojban();
                collect_cll_inlines_page_find_entries(entries, &line.body, script, line_context);
                if let Some(comment) = &line.comment {
                    collect_cll_inlines_page_find_entries(entries, comment, script, false);
                }
            }
        }
        CllBlock::LujvoMaking { parts, .. } => {
            for part in parts {
                push_page_find_entry(entries, part.kind.as_str());
                let part_context = part.kind.is_lojban();
                collect_cll_inlines_page_find_entries(entries, &part.body, script, part_context);
            }
        }
        CllBlock::Ebnf { entries: rules, .. } => {
            for rule in rules {
                push_page_find_entry(entries, rule.rule_name.clone());
                collect_cll_ebnf_tokens_page_find_entries(entries, &rule.rhs);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_table_cell_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    site: Option<&jbotci_cll::CllSite>,
    cell: &CllTableCell,
    script: GentufaScript,
) {
    for child in &cell.blocks {
        collect_cll_block_page_find_entries(entries, site, child, script);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_inlines_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    inlines: &[CllInline],
    script: GentufaScript,
    lojban_context: bool,
) {
    for inline in inlines {
        collect_cll_inline_page_find_entries(entries, inline, script, lojban_context);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_inline_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    inline: &CllInline,
    script: GentufaScript,
    lojban_context: bool,
) {
    match inline {
        CllInline::Text(text) => push_page_find_entry(
            entries,
            display_lojban_text_if(script, text, lojban_context),
        ),
        CllInline::Emphasis { language, inlines } | CllInline::Quote { language, inlines } => {
            let child_context = lojban_context || cll_language_is_lojban(language.as_deref());
            collect_cll_inlines_page_find_entries(entries, inlines, script, child_context);
        }
        CllInline::LanguageSpan {
            kind,
            language,
            inlines,
        } => {
            let child_context = lojban_context
                || *kind == CllLanguageSpanKind::JboPhrase
                || cll_language_is_lojban(language.as_deref());
            collect_cll_inlines_page_find_entries(entries, inlines, script, child_context);
        }
        CllInline::CiteTitle { inlines }
        | CllInline::Subscript { inlines }
        | CllInline::Superscript { inlines }
        | CllInline::Link { inlines, .. } => {
            collect_cll_inlines_page_find_entries(entries, inlines, script, lojban_context);
        }
        CllInline::Code(text) => push_page_find_entry(entries, text.clone()),
        CllInline::Elidable { shown, inlines, .. } => {
            if inlines.is_empty() {
                push_page_find_entry(
                    entries,
                    display_lojban_text_if(script, shown, lojban_context),
                );
            } else {
                collect_cll_inlines_page_find_entries(entries, inlines, script, lojban_context);
            }
        }
        CllInline::InlineMath { .. } | CllInline::Anchor { .. } => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_ebnf_tokens_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    tokens: &[CllEbnfToken],
) {
    let lines = wrap_ebnf_choice_lines(tokens);
    for line in lines {
        for token in line {
            match token {
                CllEbnfToken::Text { body }
                | CllEbnfToken::Operator { body }
                | CllEbnfToken::Hash { body }
                | CllEbnfToken::Terminal { body, .. }
                | CllEbnfToken::ElidableTerminator { body, .. }
                | CllEbnfToken::Nonterminal { body, .. } => {
                    if let Some((prefix, suffix)) = cll_ebnf_elidable_hash_pieces(&body) {
                        push_page_find_entry(entries, prefix.to_owned());
                        push_page_find_entry(entries, "#");
                        push_page_find_entry(entries, suffix.to_owned());
                    } else {
                        push_page_find_entry(entries, body.clone());
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    result: &VlackuWebResult,
    script: GentufaScript,
) {
    if let Some(message) = &result.message {
        push_page_find_entry(entries, semantic_search_message_visible_text(message));
    }
    for error in &result.errors {
        push_page_find_entry(entries, error.clone());
    }
    for card in &result.cards {
        collect_vlacku_card_page_find_entries(entries, card, script);
    }
    if result.has_more {
        push_page_find_entry(entries, "Load more");
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_gimfihi_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    result: &GimfihiWebResult,
) {
    push_page_find_entry(entries, "gimfi'i");
    for source in &result.state.sources {
        push_page_find_entry(entries, source.language.clone());
        push_page_find_entry(entries, source.word.clone());
        if let Some(weight) = &source.weight {
            push_page_find_entry(entries, weight.clone());
        }
    }
    for error in &result.errors {
        push_page_find_entry(entries, error.clone());
    }
    let Some(output) = &result.output else {
        return;
    };
    if let Some(winner) = &output.winner {
        push_page_find_entry(entries, winner.clone());
    }
    if let Some(highlighted) = &output.highlighted_word {
        push_page_find_entry(entries, highlighted.clone());
    }
    for candidate in &output.candidates {
        push_page_find_entry(entries, candidate.word.clone());
        push_page_find_entry(entries, format_gimfihi_score(candidate.score));
        if let Some(collision) = &candidate.collision {
            push_page_find_entry(entries, collision.existing_word.clone());
        }
        for rafsi in candidate.rafsi() {
            push_page_find_entry(entries, rafsi.form.clone());
            for source in &rafsi.taken_by {
                push_page_find_entry(entries, source.clone());
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_card_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    card: &VlackuWebCard,
    script: GentufaScript,
) {
    push_page_find_entry(entries, display_lojban_text(script, &card.display_word));
    if let Some(ipa) = &card.ipa {
        push_page_find_entry(entries, format!("/{ipa}/"));
    }
    for piece in card
        .decomposition
        .iter()
        .filter(|piece| piece.kind != VlackuCompositionPieceKind::Hyphen)
    {
        collect_vlacku_composition_piece_page_find_entries(entries, piece, script);
    }
    if card.decomposition.is_empty() {
        for rafsi in &card.rafsi {
            push_page_find_entry(entries, display_lojban_text(script, rafsi));
        }
    }
    if let Some(author) = &card.author {
        push_page_find_entry(entries, vlacku_author_credit_text(author));
    }
    push_page_find_entry(entries, card.word_type.clone());
    if let Some(selmaho) = &card.selmaho {
        push_page_find_entry(entries, selmaho.clone());
    }
    if let Some(similarity) = card.similarity {
        push_page_find_entry(entries, format_similarity(similarity));
    }
    collect_vote_display_page_find_entries(entries, &card.votes);
    collect_vlacku_inlines_page_find_entries(entries, &card.definition, script);
    for gloss in &card.glosses {
        push_page_find_entry(entries, gloss.clone());
    }
    collect_vlacku_inlines_page_find_entries(entries, &card.notes, script);
    if !card.etymology.is_empty() {
        push_page_find_entry(entries, "etymology: ");
        collect_vlacku_inlines_page_find_entries(entries, &card.etymology, script);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_composition_piece_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    piece: &VlackuCompositionPiece,
    script: GentufaScript,
) {
    if piece.kind != VlackuCompositionPieceKind::Rafsi {
        return;
    }
    push_page_find_entry(entries, display_lojban_text(script, &piece.display_surface));
    if let Some(source) = &piece.source
        && !piece.source_is_surface
    {
        let source_label = piece.display_source.as_deref().unwrap_or(source);
        push_page_find_entry(entries, display_lojban_text(script, source_label));
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vote_display_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    votes: &VlackuVoteDisplay,
) {
    match votes {
        VlackuVoteDisplay::Known(value) => push_page_find_entry(entries, value.to_string()),
        VlackuVoteDisplay::Unknown => push_page_find_entry(entries, "?"),
        VlackuVoteDisplay::Hidden => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_inlines_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    spans: &[VlackuInline],
    script: GentufaScript,
) {
    for span in spans {
        match span.as_data() {
            data!(VlackuInline::Text(text)) => push_page_find_entry(entries, text.clone()),
            data!(VlackuInline::Math(_math)) => {}
            data!(VlackuInline::WordRef { label, .. }) => {
                push_page_find_entry(entries, display_lojban_text(script, label));
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_gentufa_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    result: &GentufaWebResult,
    request: Option<&GentufaWebRequest>,
    view_mode: GentufaWebViewMode,
    display: GentufaDisplayState,
    script: GentufaScript,
) {
    match result {
        GentufaWebResult::Blank => {}
        GentufaWebResult::Error(error) => {
            collect_diagnostics_pane_page_find_entries(
                entries,
                &error.diagnostics,
                gentufa_request_source(request),
                Some(error.message.as_str()),
                true,
                script,
            );
        }
        GentufaWebResult::Success(success) => {
            collect_bracket_fragments_page_find_entries(entries, &success.bracket_fragments);
            collect_diagnostics_pane_page_find_entries(
                entries,
                &success.diagnostics,
                gentufa_request_source(request),
                None,
                true,
                script,
            );
            match view_mode {
                GentufaWebViewMode::Blocks => {
                    for block in &success.blocks_layout.blocks {
                        push_page_find_entry(entries, block.label.clone());
                    }
                    if display.show_glosses {
                        for block in success
                            .blocks_layout
                            .blocks
                            .iter()
                            .filter(|block| block.is_leaf)
                        {
                            let text = block
                                .computed_gloss
                                .as_deref()
                                .or_else(|| block.glosses.first().map(String::as_str))
                                .unwrap_or("");
                            push_page_find_entry(entries, text.to_owned());
                        }
                    }
                }
                GentufaWebViewMode::Tree => {
                    for row in &success.tree_rows {
                        push_page_find_entry(entries, row.label.clone());
                        for cell in &row.cells {
                            push_page_find_entry(entries, cell.text.clone());
                        }
                    }
                }
                GentufaWebViewMode::Ipa => push_page_find_entry(entries, success.ipa_text.clone()),
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bracket_fragments_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    fragments: &[GentufaBracketFragment],
) {
    for fragment in fragments {
        match fragment {
            GentufaBracketFragment::Text { text, .. } => {
                push_page_find_entry(entries, text.clone())
            }
            GentufaBracketFragment::Span { children, .. } => {
                collect_bracket_fragments_page_find_entries(entries, children);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_diagnostics_pane_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    diagnostics: &[Diagnostic],
    source: &str,
    fallback_error: Option<&str>,
    diagnostics_open: bool,
    script: GentufaScript,
) {
    let fallback_error = fallback_error.filter(|message| !message.is_empty());
    if diagnostics.is_empty() && fallback_error.is_none() {
        return;
    }
    push_page_find_entry(
        entries,
        diagnostic_pane_title(diagnostic_counts(diagnostics, fallback_error)),
    );
    push_page_find_entry(entries, diagnostics_toggle_label(diagnostics_open));
    if !diagnostics_open {
        return;
    }
    if diagnostics.is_empty() {
        if let Some(message) = fallback_error {
            push_page_find_entry(entries, "error");
            push_page_find_entry(entries, message.to_owned());
        }
        return;
    }
    for diagnostic in diagnostics {
        collect_diagnostic_card_page_find_entries(entries, diagnostic, source, script);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_diagnostic_card_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    diagnostic: &Diagnostic,
    source: &str,
    script: GentufaScript,
) {
    push_page_find_entry(entries, diagnostic_severity_text(diagnostic.severity));
    push_page_find_entry(entries, diagnostic.code.clone());
    let location = diagnostic_label_location(source, diagnostic.primary_label());
    push_page_find_entry(
        entries,
        format!(
            "{}:{}: {}",
            location.line, location.column, diagnostic.message
        ),
    );
    for label in diagnostic_context_labels(diagnostic) {
        if let Some(descriptor) = diagnostic_context_descriptor(&label.message) {
            push_page_find_entry(entries, "while parsing ");
            push_page_find_entry(entries, descriptor);
        } else {
            push_page_find_entry(entries, label.message.clone());
        }
    }
    for segment in diagnostic_primary_detail_parts(diagnostic) {
        push_page_find_entry(
            entries,
            diagnostic_display_text_part_for_script(&segment, script),
        );
    }
    for note in diagnostic_plain_note_segments_for_web(diagnostic) {
        for segment in note {
            push_page_find_entry(
                entries,
                diagnostic_display_text_part_for_script(&segment, script),
            );
        }
    }
    for note in diagnostic_styled_notes_for_web(diagnostic) {
        for part in diagnostic_text_segment_render_parts(&note.segments) {
            push_page_find_entry(
                entries,
                diagnostic_display_text_part_for_script(&part, script),
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_settings_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    _current_settings: UserSettings,
    dialect_settings: &DialectSettings,
    selected_dialect: &str,
    embedding_settings: &EmbeddingSettingsState,
) {
    push_page_find_entry(entries, "Settings");
    if let Some(commit) = build_commit_info() {
        push_page_find_entry(entries, format!("commit {}", commit.short));
    }
    push_page_find_entry(entries, "Semantic search");
    push_page_find_entry(entries, "Embedding model");
    push_page_find_entry(entries, "Status");
    push_page_find_entry(entries, embedding_settings.status.clone());
    push_page_find_entry(entries, "Model");
    push_page_find_entry(entries, embedding_settings.model_size.clone());
    push_page_find_entry(entries, "Index");
    push_page_find_entry(entries, embedding_settings.index_size.clone());
    push_page_find_entry(entries, embedding_settings.detail.clone());
    if embedding_settings.busy || embedding_settings.progress_percent.is_some() {
        push_page_find_entry(
            entries,
            embedding_progress_display_label(embedding_settings),
        );
    }
    push_page_find_entry(entries, "Download");
    push_page_find_entry(entries, "Update");
    push_page_find_entry(entries, "Remove");
    push_page_find_entry(entries, "Parsing");
    push_page_find_entry(entries, "Error context depth");
    if embedding_settings.remove_confirmation_open {
        push_page_find_entry(
            entries,
            format!("Remove {}", embedding_settings.selected_model_label),
        );
        push_page_find_entry(
            entries,
            "This will remove the selected model files and vector index from this device.",
        );
        push_page_find_entry(entries, "Cancel");
        push_page_find_entry(entries, "Remove");
    }
    push_page_find_entry(entries, "Output");
    push_page_find_entry(entries, "Stress");
    push_page_find_entry(entries, "none");
    push_page_find_entry(entries, "acute");
    push_page_find_entry(entries, "caps");
    push_page_find_entry(entries, "Glides");
    push_page_find_entry(entries, "none");
    push_page_find_entry(entries, "breve");
    push_page_find_entry(entries, "Lojban dialects");
    push_page_find_entry(entries, "Builtins");
    for name in builtin_dialect_names() {
        push_page_find_entry(entries, name);
    }
    push_page_find_entry(entries, "Custom");
    for custom in &dialect_settings.custom_dialects {
        let item_name = custom.name.trim();
        push_page_find_entry(
            entries,
            if item_name.is_empty() {
                "(unnamed)"
            } else {
                item_name
            },
        );
    }
    if selected_dialect.trim().is_empty() {
        push_page_find_entry(entries, "Select a dialect to edit it.");
    } else {
        push_page_find_entry(entries, "Name");
        push_page_find_entry(entries, "Show in gentufa");
        push_page_find_entry(entries, "Definition");
        if let Some(custom) = dialect_settings
            .custom_dialects
            .iter()
            .find(|custom| custom.name.trim() == selected_dialect.trim())
            && let Err(error) = custom_dialect_is_valid(&dialect_settings.custom_dialects, custom)
        {
            push_page_find_entry(entries, error.message().to_owned());
        } else {
            push_page_find_entry(entries, "Definition is valid.");
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn semantic_search_message_visible_text(message: &str) -> String {
    if message == SEMANTIC_SEARCH_SETUP_MESSAGE {
        format!("{SEMANTIC_SEARCH_SETUP_LINK_LABEL}{SEMANTIC_SEARCH_SETUP_LINK_SUFFIX}")
    } else {
        message.to_owned()
    }
}


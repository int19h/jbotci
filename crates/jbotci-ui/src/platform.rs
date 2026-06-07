use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use serde::{Deserialize, Serialize};

pub type PlatformFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct PlatformServiceError {
    pub message: String,
}

impl PlatformServiceError {
    #[requires(!message.is_empty())]
    #[ensures(!ret.message.is_empty())]
    pub fn new(message: String) -> Self {
        Self { message }
    }

    #[requires(!service_name.is_empty())]
    #[ensures(ret.message.contains(service_name))]
    pub fn unsupported(service_name: &str) -> Self {
        Self {
            message: format!("{service_name} is not available on this platform yet"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(::Available => true)]
#[invariant(::Unavailable { .. } => true)]
pub enum PlatformAvailability {
    Available,
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingStatus {
    pub availability: PlatformAvailability,
    pub model_key: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingSetupProgress {
    pub model_key: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingSearchRequest {
    pub corpus: String,
    pub query: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingSearchResponse {
    pub json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub enum ExportFormat {
    Svg,
    Png,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct ExportRequest {
    pub suggested_name: String,
    pub format: ExportFormat,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    #[requires(true)]
    #[ensures(ret == self.left + self.width)]
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    #[requires(true)]
    #[ensures(ret == self.top + self.height)]
    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TooltipPlacement {
    pub left: f64,
    pub top: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub enum SharedTopbarSettingsLayout {
    BothInline,
    ThemeInline,
    NoneInline,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TopbarLayoutMetrics {
    pub available_width: f64,
    pub both_probe_width: f64,
    pub theme_probe_width: f64,
    pub center_width: f64,
    pub right_width: f64,
    pub column_gap: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TreeLine {
    pub x: f64,
    pub start_y: f64,
    pub end_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TreeLineAnchor {
    pub node_id: usize,
    pub parent_id: Option<usize>,
    pub depth: usize,
    pub label_left: f64,
    pub label_center_y: f64,
    pub row_top: f64,
    pub row_bottom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
pub struct JvozbaPaneLayout {
    pub top: f64,
    pub bottom: f64,
    pub height: f64,
    pub scrollbar_gutter_width: i32,
    pub topbar_bottom: f64,
}

#[contract_trait]
pub trait ComputeService {
    #[requires(!request_json.is_empty())]
    #[ensures(true)]
    fn compute_json<'a>(
        &'a self,
        request_json: &'a str,
    ) -> PlatformFuture<'a, Result<String, PlatformServiceError>>;
}

#[contract_trait]
pub trait EmbeddingService {
    #[requires(true)]
    #[ensures(true)]
    fn status<'a>(&'a self) -> PlatformFuture<'a, Result<EmbeddingStatus, PlatformServiceError>>;

    #[requires(!model_key.is_empty())]
    #[ensures(true)]
    fn setup<'a>(
        &'a self,
        model_key: &'a str,
    ) -> PlatformFuture<'a, Result<EmbeddingSetupProgress, PlatformServiceError>>;

    #[requires(true)]
    #[ensures(true)]
    fn remove<'a>(&'a self) -> PlatformFuture<'a, Result<(), PlatformServiceError>>;

    #[requires(!request.query.is_empty())]
    #[ensures(true)]
    fn search<'a>(
        &'a self,
        request: EmbeddingSearchRequest,
    ) -> PlatformFuture<'a, Result<EmbeddingSearchResponse, PlatformServiceError>>;
}

#[contract_trait]
pub trait SettingsStore {
    #[requires(!key.is_empty())]
    #[ensures(true)]
    fn get(&self, key: &str) -> Option<String>;

    #[requires(!key.is_empty())]
    #[ensures(true)]
    fn set(&self, key: &str, value: &str) -> Result<(), PlatformServiceError>;
}

#[contract_trait]
pub trait ClipboardService {
    #[requires(true)]
    #[ensures(true)]
    fn copy_text<'a>(
        &'a self,
        text: &'a str,
    ) -> PlatformFuture<'a, Result<(), PlatformServiceError>>;
}

#[contract_trait]
pub trait ExportService {
    #[requires(!request.suggested_name.is_empty())]
    #[ensures(true)]
    fn export<'a>(
        &'a self,
        request: ExportRequest,
    ) -> PlatformFuture<'a, Result<(), PlatformServiceError>>;
}

#[derive(Debug, Default)]
#[invariant(true)]
pub struct NativeComputeService;

#[cfg(not(target_arch = "wasm32"))]
#[contract_trait]
impl ComputeService for NativeComputeService {
    fn compute_json<'a>(
        &'a self,
        request_json: &'a str,
    ) -> PlatformFuture<'a, Result<String, PlatformServiceError>> {
        Box::pin(async move {
            jbotci_web_core::run_web_compute_request_json(request_json)
                .map_err(|error| PlatformServiceError::new(error.to_string()))
        })
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct UnsupportedEmbeddingService {
    reason: String,
}

impl UnsupportedEmbeddingService {
    #[requires(!reason.is_empty())]
    #[ensures(!ret.reason.is_empty())]
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[contract_trait]
impl EmbeddingService for UnsupportedEmbeddingService {
    fn status<'a>(&'a self) -> PlatformFuture<'a, Result<EmbeddingStatus, PlatformServiceError>> {
        Box::pin(async move {
            Ok(EmbeddingStatus {
                availability: PlatformAvailability::Unavailable {
                    reason: self.reason.clone(),
                },
                model_key: String::new(),
                detail: self.reason.clone(),
            })
        })
    }

    fn setup<'a>(
        &'a self,
        _model_key: &'a str,
    ) -> PlatformFuture<'a, Result<EmbeddingSetupProgress, PlatformServiceError>> {
        Box::pin(async move { Err(PlatformServiceError::unsupported("embedding setup")) })
    }

    fn remove<'a>(&'a self) -> PlatformFuture<'a, Result<(), PlatformServiceError>> {
        Box::pin(async move { Err(PlatformServiceError::unsupported("embedding removal")) })
    }

    fn search<'a>(
        &'a self,
        _request: EmbeddingSearchRequest,
    ) -> PlatformFuture<'a, Result<EmbeddingSearchResponse, PlatformServiceError>> {
        Box::pin(async move { Err(PlatformServiceError::unsupported("semantic search")) })
    }
}

#[derive(Debug, Default)]
#[invariant(true)]
pub struct MemorySettingsStore {
    values: RefCell<HashMap<String, String>>,
}

#[contract_trait]
impl SettingsStore for MemorySettingsStore {
    fn get(&self, key: &str) -> Option<String> {
        self.values.borrow().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), PlatformServiceError> {
        self.values
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }
}

#[derive(Debug, Default)]
#[invariant(true)]
pub struct UnsupportedClipboardService;

#[contract_trait]
impl ClipboardService for UnsupportedClipboardService {
    fn copy_text<'a>(
        &'a self,
        _text: &'a str,
    ) -> PlatformFuture<'a, Result<(), PlatformServiceError>> {
        Box::pin(async move { Err(PlatformServiceError::unsupported("clipboard")) })
    }
}

#[derive(Debug, Default)]
#[invariant(true)]
pub struct UnsupportedExportService;

#[contract_trait]
impl ExportService for UnsupportedExportService {
    fn export<'a>(
        &'a self,
        _request: ExportRequest,
    ) -> PlatformFuture<'a, Result<(), PlatformServiceError>> {
        Box::pin(async move { Err(PlatformServiceError::unsupported("export")) })
    }
}

#[requires(host.width >= 0.0)]
#[requires(host.height >= 0.0)]
#[requires(tooltip.width >= 0.0)]
#[requires(tooltip.height >= 0.0)]
#[requires(viewport.width >= 0.0)]
#[requires(viewport.height >= 0.0)]
#[ensures(ret.left >= margin.max(0.0))]
#[ensures(ret.top >= margin.max(0.0))]
pub fn place_tooltip(
    host: Rect,
    tooltip: Size,
    viewport: Viewport,
    margin: f64,
    gap: f64,
) -> TooltipPlacement {
    let margin = margin.max(0.0);
    let gap = gap.max(0.0);
    let tooltip_width = tooltip.width.max(1.0);
    let tooltip_height = tooltip.height.max(1.0);
    let max_left = (viewport.width - tooltip_width - margin).max(margin);
    let centered_left = host.left + host.width / 2.0 - tooltip_width / 2.0;
    let left = centered_left.clamp(margin, max_left);
    let preferred_top = host.top - tooltip_height - gap;
    let max_top = (viewport.height - tooltip_height - margin).max(margin);
    let top = if preferred_top >= margin {
        preferred_top.min(max_top)
    } else {
        (host.bottom() + gap).clamp(margin, max_top)
    };
    TooltipPlacement { left, top }
}

#[requires(metrics.available_width >= 0.0)]
#[ensures(true)]
pub fn choose_topbar_layout(metrics: TopbarLayoutMetrics) -> SharedTopbarSettingsLayout {
    if topbar_probe_fits(metrics, metrics.both_probe_width) {
        SharedTopbarSettingsLayout::BothInline
    } else if topbar_probe_fits(metrics, metrics.theme_probe_width) {
        SharedTopbarSettingsLayout::ThemeInline
    } else {
        SharedTopbarSettingsLayout::NoneInline
    }
}

#[requires(probe_width >= 0.0)]
#[ensures(true)]
fn topbar_probe_fits(metrics: TopbarLayoutMetrics, probe_width: f64) -> bool {
    let visible_columns = 1.0
        + if metrics.center_width > 0.0 { 1.0 } else { 0.0 }
        + if metrics.right_width > 0.0 { 1.0 } else { 0.0 };
    let required_width = probe_width
        + metrics.center_width
        + metrics.right_width
        + (visible_columns - 1.0) * metrics.column_gap;
    required_width <= metrics.available_width + 1.0
}

#[requires(table_bottom.is_finite())]
#[ensures(ret.iter().all(|line| line.end_y >= line.start_y))]
pub fn gentufa_tree_lines(anchors: &[TreeLineAnchor], table_bottom: f64) -> Vec<TreeLine> {
    let mut lines = Vec::new();
    for (index, anchor) in anchors.iter().enumerate() {
        if !anchors
            .iter()
            .any(|candidate| candidate.parent_id == Some(anchor.node_id))
        {
            continue;
        }
        let end_y = anchors
            .iter()
            .skip(index + 1)
            .find_map(|candidate| (candidate.depth <= anchor.depth).then_some(candidate.row_top))
            .unwrap_or(table_bottom.max(anchor.row_bottom));
        if end_y > anchor.label_center_y {
            lines.push(TreeLine {
                x: anchor.label_left,
                start_y: anchor.label_center_y,
                end_y,
            });
        }
    }
    lines
}

#[requires(scroll_top >= 0)]
#[requires(fallback_top.is_finite())]
#[requires(topbar_bottom.is_finite())]
#[ensures(ret.is_finite())]
#[ensures(ret >= topbar_bottom)]
pub fn stable_jvozba_pane_top(
    anchor_viewport_top: Option<f64>,
    scroll_top: i32,
    fallback_top: f64,
    topbar_bottom: f64,
) -> f64 {
    anchor_viewport_top
        .map(|top| top + f64::from(scroll_top))
        .unwrap_or(fallback_top)
        .max(topbar_bottom)
}

#[requires(viewport_height >= 0.0)]
#[requires(scroll_top >= 0)]
#[requires(scrollbar_gutter_width >= 0)]
#[ensures(ret.height >= 0.0)]
pub fn compute_jvozba_pane_layout(
    anchor_viewport_top: Option<f64>,
    scroll_top: i32,
    fallback_top: f64,
    topbar_bottom: f64,
    viewport_height: f64,
    scrollbar_gutter_width: i32,
    height_scale: f64,
) -> JvozbaPaneLayout {
    let top = stable_jvozba_pane_top(anchor_viewport_top, scroll_top, fallback_top, topbar_bottom);
    let bottom = 12.0;
    let height = (viewport_height - top - bottom).max(280.0) * height_scale.max(0.0);
    JvozbaPaneLayout {
        top,
        bottom,
        height,
        scrollbar_gutter_width,
        topbar_bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tooltip_placement_clamps_to_viewport() {
        let placement = place_tooltip(
            Rect {
                left: 2.0,
                top: 4.0,
                width: 10.0,
                height: 8.0,
            },
            Size {
                width: 40.0,
                height: 20.0,
            },
            Viewport {
                width: 50.0,
                height: 50.0,
            },
            8.0,
            8.0,
        );
        assert_eq!(placement.left, 8.0);
        assert_eq!(placement.top, 20.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn topbar_layout_chooses_first_fitting_probe() {
        let metrics = TopbarLayoutMetrics {
            available_width: 200.0,
            both_probe_width: 150.0,
            theme_probe_width: 100.0,
            center_width: 40.0,
            right_width: 20.0,
            column_gap: 8.0,
        };
        assert_eq!(
            choose_topbar_layout(metrics),
            SharedTopbarSettingsLayout::ThemeInline
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_lines_stop_at_next_shallower_row() {
        let anchors = vec![
            TreeLineAnchor {
                node_id: 1,
                parent_id: None,
                depth: 0,
                label_left: 10.0,
                label_center_y: 10.0,
                row_top: 0.0,
                row_bottom: 20.0,
            },
            TreeLineAnchor {
                node_id: 2,
                parent_id: Some(1),
                depth: 1,
                label_left: 20.0,
                label_center_y: 30.0,
                row_top: 20.0,
                row_bottom: 40.0,
            },
            TreeLineAnchor {
                node_id: 3,
                parent_id: None,
                depth: 0,
                label_left: 10.0,
                label_center_y: 50.0,
                row_top: 40.0,
                row_bottom: 60.0,
            },
        ];
        assert_eq!(
            gentufa_tree_lines(&anchors, 60.0),
            vec![TreeLine {
                x: 10.0,
                start_y: 10.0,
                end_y: 40.0,
            }]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_layout_uses_stable_unscrolled_anchor() {
        let layout = compute_jvozba_pane_layout(Some(-658.0), 900, 46.0, 34.0, 900.0, 15, 0.72);
        assert_eq!(layout.top, 242.0);
        assert_eq!(layout.scrollbar_gutter_width, 15);
        assert!(layout.height > 0.0);
    }
}

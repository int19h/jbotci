use std::collections::HashMap;

use base64::Engine;
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use thiserror::Error;
use xmlwriter::{Indent, Options as XmlOptions, XmlWriter};

#[cfg(test)]
use crate::ReferenceSlotLabel;
use crate::{
    GentufaBlock, GentufaBlocksLayout, GentufaScript, ReferenceLabel, ReferenceMarker,
    ReferenceMarkerRole, math_alphanumeric_stem, reference_slot_display_text,
};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const OUTER_PADDING: f32 = 12.0;
const BLOCK_GAP: f32 = 1.0;
const BLOCK_PADDING: f32 = 11.2;
const BLOCK_LABEL_BOTTOM_PADDING: f32 = 15.2;
const NONLEAF_LABEL_BOTTOM_PADDING: f32 = 7.1;
const REF_PAD_X: f32 = 4.0;
const REF_PAD_Y: f32 = 1.0;
const REF_LINE_GAP: f32 = 1.3;
const BLOCK_REFERENCE_LABEL_GAP: f32 = 8.0;
const ROW_COMPACT_HEIGHT: f32 = 32.0;
const GLOSS_ROW_HEIGHT: f32 = 55.2;
const MIN_COLUMN_WIDTH: f32 = 44.0;
const INK: &str = "#231b15";
const GLOSS_INK: &str = "#6f6257";
const GLOSS_BG: &str = "#ece3d7";
pub const DEFAULT_GENTUFA_PNG_SCALE: f32 = 2.0;

#[derive(Debug, Error)]
#[invariant(true)]
#[invariant(::Xml(_) => true)]
#[invariant(::Svg(_) => true)]
#[invariant(::Png(_) => true)]
#[invariant(::InvalidSize => true)]
pub enum GentufaExportError {
    #[error("failed to parse generated SVG XML: {0}")]
    Xml(String),
    #[error("failed to parse generated SVG for rendering: {0}")]
    Svg(String),
    #[error("failed to encode PNG: {0}")]
    Png(String),
    #[error("generated SVG has an invalid raster size")]
    InvalidSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct GentufaFontData<'a> {
    pub noto_sans: &'a [u8],
    pub noto_sans_italic: &'a [u8],
    pub stix_two_math: &'a [u8],
    pub stix_two_text: &'a [u8],
    pub stix_two_text_bold: &'a [u8],
    pub crisa: Option<&'a [u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct EmbeddedGentufaFonts;

impl EmbeddedGentufaFonts {
    #[requires(true)]
    #[ensures(!ret.noto_sans.is_empty())]
    pub fn get() -> GentufaFontData<'static> {
        GentufaFontData {
            noto_sans: include_bytes!(
                "../../../apps/jbotci-web/assets/fonts/noto-sans-variable.ttf"
            ),
            noto_sans_italic: include_bytes!(
                "../../../apps/jbotci-web/assets/fonts/noto-sans-italic-variable.ttf"
            ),
            stix_two_math: include_bytes!(
                "../../../apps/jbotci-web/assets/fonts/stix-two-math-regular.ttf"
            ),
            stix_two_text: include_bytes!(
                "../../../apps/jbotci-web/assets/fonts/stix-two-text-regular.ttf"
            ),
            stix_two_text_bold: include_bytes!(
                "../../../apps/jbotci-web/assets/fonts/stix-two-text-bold.ttf"
            ),
            crisa: Some(include_bytes!(
                "../../../apps/jbotci-web/assets/fonts/crisa-regular.otf"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct GentufaSvgOptions {
    pub show_glosses: bool,
    pub script: GentufaScript,
    pub title: String,
}

impl Default for GentufaSvgOptions {
    #[requires(true)]
    #[ensures(ret.show_glosses)]
    fn default() -> Self {
        Self {
            show_glosses: true,
            script: GentufaScript::Latin,
            title: "jbotci gentufa blocks".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct GentufaPngOptions {
    pub svg: GentufaSvgOptions,
    pub scale: f32,
}

impl Default for GentufaPngOptions {
    #[requires(true)]
    #[ensures(ret.scale == DEFAULT_GENTUFA_PNG_SCALE)]
    fn default() -> Self {
        Self {
            svg: GentufaSvgOptions::default(),
            scale: DEFAULT_GENTUFA_PNG_SCALE,
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|svg| svg.contains("<svg")) || ret.is_err())]
pub fn render_gentufa_blocks_svg<Tooltip>(
    layout: &GentufaBlocksLayout<Tooltip>,
    options: &GentufaSvgOptions,
    fonts: GentufaFontData<'_>,
) -> Result<String, GentufaExportError> {
    let mut measurer = TextMeasurer::new(fonts);
    let positioned = PositionedBlocks::new(layout, options, &mut measurer)?;
    let document = svg_document(layout, options, fonts, &positioned, &mut measurer)?;
    Ok(document.to_xml())
}

#[requires(options.scale.is_finite() && options.scale > 0.0)]
#[ensures(ret.as_ref().is_ok_and(|png| png.starts_with(b"\x89PNG\r\n\x1a\n")) || ret.is_err())]
pub fn render_gentufa_blocks_png<Tooltip>(
    layout: &GentufaBlocksLayout<Tooltip>,
    options: &GentufaPngOptions,
    fonts: GentufaFontData<'_>,
) -> Result<Vec<u8>, GentufaExportError> {
    let svg = render_gentufa_blocks_svg(layout, &options.svg, fonts)?;
    let xml = roxmltree::Document::parse(&svg)
        .map_err(|error| GentufaExportError::Xml(error.to_string()))?;
    let usvg_options = usvg_options(fonts);
    let tree = usvg::Tree::from_xmltree(&xml, &usvg_options)
        .map_err(|error| GentufaExportError::Svg(error.to_string()))?;
    let size = tree.size();
    let width = (size.width() * options.scale).ceil() as u32;
    let height = (size.height() * options.scale).ceil() as u32;
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or(GentufaExportError::InvalidSize)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(options.scale, options.scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| GentufaExportError::Png(error.to_string()))
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct TextSize {
    width: f32,
    height: f32,
    baseline_top: f32,
    baseline_bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
struct ReferenceStackBottoms {
    stack: f32,
    overlapping_label: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[invariant(true)]
enum TextRole {
    LeafLabel,
    NonleafLabel,
    Reference,
    Gloss,
}

impl TextRole {
    #[requires(true)]
    #[ensures(ret > 0.0)]
    fn font_size(self) -> f32 {
        match self {
            Self::LeafLabel => 16.0,
            Self::NonleafLabel => 12.8,
            Self::Reference | Self::Gloss => 11.2,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn font_weight(self) -> &'static str {
        match self {
            Self::LeafLabel | Self::Reference => "700",
            Self::NonleafLabel | Self::Gloss => "400",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn font_style(self) -> &'static str {
        match self {
            Self::NonleafLabel => "italic",
            Self::LeafLabel | Self::Reference | Self::Gloss => "normal",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn font_family(self, script: GentufaScript) -> &'static str {
        match (self, script) {
            (Self::LeafLabel | Self::NonleafLabel, GentufaScript::Zbalermorna) => "Crisa",
            (Self::Reference, _) => "STIX Two Text, STIX Two Math, serif",
            _ => "Noto Sans, STIX Two Math, sans-serif",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[invariant(true)]
struct TextMeasureKey {
    text: String,
    role: TextRole,
    script: GentufaScript,
}

#[derive(Debug)]
#[invariant(true)]
struct TextMeasurer {
    options: usvg::Options<'static>,
    cache: HashMap<TextMeasureKey, TextSize>,
}

impl TextMeasurer {
    #[requires(true)]
    #[ensures(true)]
    fn new(fonts: GentufaFontData<'_>) -> Self {
        Self {
            options: usvg_options(fonts),
            cache: HashMap::new(),
        }
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|size| size.width >= 0.0 && size.height >= 0.0) || ret.is_err())]
    fn measure(
        &mut self,
        text: &str,
        role: TextRole,
        script: GentufaScript,
    ) -> Result<TextSize, GentufaExportError> {
        let key = TextMeasureKey {
            text: text.to_owned(),
            role,
            script,
        };
        if let Some(size) = self.cache.get(&key) {
            return Ok(size.clone());
        }
        let size = measure_text_with_usvg(text, role, script, &self.options)?;
        self.cache.insert(key, size.clone());
        Ok(size)
    }
}

#[requires(!text.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|size| size.width >= 0.0 && size.height >= 0.0) || ret.is_err())]
fn measure_text_with_usvg(
    text: &str,
    role: TextRole,
    script: GentufaScript,
    options: &usvg::Options<'_>,
) -> Result<TextSize, GentufaExportError> {
    let mut root = SvgElement::new(SvgTag::Svg);
    root.attr("xmlns", SVG_NS);
    root.attr("width", "10000");
    root.attr("height", "1000");
    root.attr("viewBox", "0 0 10000 1000");
    let mut text_node = SvgElement::new(SvgTag::Text);
    text_node.attr("id", "measure-text");
    text_node.attr("x", "0");
    text_node.attr("y", "200");
    text_node.attr("font-family", role.font_family(script));
    text_node.attr("font-size", &format_float(role.font_size()));
    text_node.attr("font-weight", role.font_weight());
    text_node.attr("font-style", role.font_style());
    text_node.text(text);
    root.child(text_node);
    let svg = SvgDocument { root }.to_xml();
    let xml = roxmltree::Document::parse(&svg)
        .map_err(|error| GentufaExportError::Xml(error.to_string()))?;
    let tree = usvg::Tree::from_xmltree(&xml, options)
        .map_err(|error| GentufaExportError::Svg(error.to_string()))?;
    let Some(node) = tree.node_by_id("measure-text") else {
        return Ok(TextSize {
            width: 0.0,
            height: role.font_size(),
            baseline_top: -role.font_size(),
            baseline_bottom: 0.0,
        });
    };
    let bbox = node.bounding_box();
    Ok(TextSize {
        width: bbox.width(),
        height: bbox.height(),
        baseline_top: bbox.top() - 200.0,
        baseline_bottom: bbox.bottom() - 200.0,
    })
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct PositionedBlocks {
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    gloss_row_height: Option<f32>,
    width: f32,
    height: f32,
}

impl PositionedBlocks {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|layout| layout.width > 0.0 && layout.height > 0.0) || ret.is_err())]
    fn new<Tooltip>(
        layout: &GentufaBlocksLayout<Tooltip>,
        options: &GentufaSvgOptions,
        measurer: &mut TextMeasurer,
    ) -> Result<Self, GentufaExportError> {
        let column_count = layout.max_col.max(1);
        let row_count = layout.max_row.max(1);
        let mut column_widths = vec![MIN_COLUMN_WIDTH; column_count];
        grow_columns_for_blocks(&mut column_widths, &layout.blocks, options, measurer)?;
        let mut row_heights = vec![ROW_COMPACT_HEIGHT; row_count];
        grow_rows_for_leaf_labels(&mut row_heights, &layout.blocks, options, measurer)?;
        grow_rows_for_references(
            &mut row_heights,
            &column_widths,
            &layout.blocks,
            options,
            measurer,
        )?;
        let gloss_row_height = if options.show_glosses {
            Some(gloss_row_height(&layout.blocks, options, measurer)?)
        } else {
            None
        };
        let width = OUTER_PADDING * 2.0
            + column_widths.iter().sum::<f32>()
            + BLOCK_GAP * column_count.saturating_sub(1) as f32;
        let height = OUTER_PADDING * 2.0
            + row_heights.iter().sum::<f32>()
            + gloss_row_height.unwrap_or(0.0)
            + BLOCK_GAP
                * (row_count + usize::from(gloss_row_height.is_some())).saturating_sub(1) as f32;
        Ok(Self {
            column_widths,
            row_heights,
            gloss_row_height,
            width,
            height,
        })
    }

    #[requires(col < self.column_widths.len())]
    #[ensures(ret >= OUTER_PADDING)]
    fn col_x(&self, col: usize) -> f32 {
        OUTER_PADDING + self.column_widths[..col].iter().sum::<f32>() + BLOCK_GAP * col as f32
    }

    #[requires(row < self.row_heights.len())]
    #[ensures(ret >= OUTER_PADDING)]
    fn row_y(&self, row: usize) -> f32 {
        OUTER_PADDING + self.row_heights[..row].iter().sum::<f32>() + BLOCK_GAP * row as f32
    }

    #[requires(col < self.column_widths.len())]
    #[requires(col_span > 0)]
    #[ensures(ret > 0.0)]
    fn span_width(&self, col: usize, col_span: usize) -> f32 {
        let end = (col + col_span).min(self.column_widths.len());
        self.column_widths[col..end].iter().sum::<f32>()
            + BLOCK_GAP * end.saturating_sub(col + 1) as f32
    }

    #[requires(row < self.row_heights.len())]
    #[requires(row_span > 0)]
    #[ensures(ret > 0.0)]
    fn span_height(&self, row: usize, row_span: usize) -> f32 {
        let end = (row + row_span).min(self.row_heights.len());
        self.row_heights[row..end].iter().sum::<f32>()
            + BLOCK_GAP * end.saturating_sub(row + 1) as f32
    }

    #[requires(true)]
    #[ensures(ret >= OUTER_PADDING)]
    fn gloss_y(&self) -> f32 {
        OUTER_PADDING
            + self.row_heights.iter().sum::<f32>()
            + BLOCK_GAP * self.row_heights.len() as f32
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn grow_rows_for_leaf_labels<Tooltip>(
    row_heights: &mut [f32],
    blocks: &[GentufaBlock<Tooltip>],
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<(), GentufaExportError> {
    let mut sorted = blocks.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|block| (block_bottom_row(block), block.row));
    for block in sorted {
        let bottom_row = block_bottom_row(block);
        if bottom_row >= row_heights.len() {
            continue;
        }
        let deficit = block_leaf_label_height_growth(block, row_heights, options, measurer)?;
        if deficit > 0.0 {
            row_heights[bottom_row] += deficit;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret >= block.row)]
fn block_bottom_row<Tooltip>(block: &GentufaBlock<Tooltip>) -> usize {
    block.row + block.row_span.saturating_sub(1)
}

#[requires(true)]
#[ensures(true)]
fn block_has_incoming_references<Tooltip>(block: &GentufaBlock<Tooltip>) -> bool {
    block
        .ref_markers
        .iter()
        .any(|marker| marker.role == ReferenceMarkerRole::Referent)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn grow_rows_for_references<Tooltip>(
    row_heights: &mut [f32],
    column_widths: &[f32],
    blocks: &[GentufaBlock<Tooltip>],
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<(), GentufaExportError> {
    let mut sorted = blocks.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|block| (block_bottom_row(block), block.row));
    for block in sorted {
        let bottom_row = block_bottom_row(block);
        if bottom_row >= row_heights.len() {
            continue;
        }
        let deficit =
            block_reference_height_growth(block, row_heights, column_widths, options, measurer)?;
        if deficit > 0.0 {
            row_heights[bottom_row] += deficit;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|growth| *growth >= 0.0) || ret.is_err())]
fn block_leaf_label_height_growth<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    row_heights: &[f32],
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<f32, GentufaExportError> {
    if !block.is_leaf || block.label.is_empty() {
        return Ok(0.0);
    }
    let span_height = block_span_height(row_heights, block.row, block.row_span);
    if span_height <= 0.0 {
        return Ok(0.0);
    }
    let label_size = measurer.measure(&block.label, TextRole::LeafLabel, options.script)?;
    let label_top = leaf_label_baseline_y(span_height) + label_size.baseline_top;
    Ok((BLOCK_PADDING - label_top).max(0.0))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|growth| *growth >= 0.0) || ret.is_err())]
fn block_reference_height_growth<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    row_heights: &[f32],
    column_widths: &[f32],
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<f32, GentufaExportError> {
    if !block_has_incoming_references(block) || block.label.is_empty() {
        return Ok(0.0);
    }
    let span_height = block_span_height(row_heights, block.row, block.row_span);
    if span_height <= 0.0 {
        return Ok(0.0);
    }
    let span_width = block_span_width(column_widths, block.col, block.col_span);
    if span_width <= 0.0 {
        return Ok(0.0);
    }
    let label_role = if block.is_leaf {
        TextRole::LeafLabel
    } else {
        TextRole::NonleafLabel
    };
    let label_size = measurer.measure(&block.label, label_role, options.script)?;
    let label_top = block_label_baseline_y(block.is_leaf, span_height) + label_size.baseline_top;
    let label_left = (span_width - label_size.width) / 2.0;
    let label_right = label_left + label_size.width;
    let Some(reference_bottoms) =
        reference_stack_bottoms(block, label_left, label_right, options, measurer)?
    else {
        return Ok(0.0);
    };
    let containment_deficit = reference_containment_deficit(reference_bottoms.stack, span_height);
    let label_deficit = reference_bottoms
        .overlapping_label
        .map(|reference_bottom| (reference_bottom + BLOCK_REFERENCE_LABEL_GAP - label_top).max(0.0))
        .unwrap_or(0.0);
    Ok(containment_deficit.max(label_deficit))
}

#[requires(span_height >= 0.0)]
#[ensures(ret <= span_height)]
fn leaf_label_baseline_y(span_height: f32) -> f32 {
    span_height - BLOCK_LABEL_BOTTOM_PADDING
}

#[requires(span_height >= 0.0)]
#[ensures(ret <= span_height)]
fn block_label_baseline_y(is_leaf: bool, span_height: f32) -> f32 {
    if is_leaf {
        leaf_label_baseline_y(span_height)
    } else {
        span_height - NONLEAF_LABEL_BOTTOM_PADDING
    }
}

#[requires(col_span > 0)]
#[ensures(ret >= 0.0)]
fn block_span_width(column_widths: &[f32], col: usize, col_span: usize) -> f32 {
    if col >= column_widths.len() {
        return 0.0;
    }
    let end = (col + col_span).min(column_widths.len());
    column_widths[col..end].iter().sum::<f32>() + BLOCK_GAP * end.saturating_sub(col + 1) as f32
}

#[requires(row_span > 0)]
#[ensures(ret >= 0.0)]
fn block_span_height(row_heights: &[f32], row: usize, row_span: usize) -> f32 {
    if row >= row_heights.len() {
        return 0.0;
    }
    let end = (row + row_span).min(row_heights.len());
    row_heights[row..end].iter().sum::<f32>() + BLOCK_GAP * end.saturating_sub(row + 1) as f32
}

#[requires(true)]
#[ensures(ret >= 0.0)]
fn reference_containment_deficit(reference_bottom: f32, span_height: f32) -> f32 {
    (reference_bottom + REF_PAD_Y - span_height).max(0.0)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bottoms| bottoms.is_none_or(|value| value.stack >= 0.0)) || ret.is_err())]
fn reference_stack_bottoms<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    label_left: f32,
    label_right: f32,
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<Option<ReferenceStackBottoms>, GentufaExportError> {
    let mut line_sizes = Vec::new();
    let line_advance = TextRole::Reference.font_size() + REF_LINE_GAP;
    for marker in block
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent)
    {
        let measured = measurer.measure(
            &reference_label_text(&marker.label),
            TextRole::Reference,
            options.script,
        )?;
        line_sizes.push(measured);
    }
    let mut stack = None;
    let mut overlapping_label = None;
    for (line, size) in line_sizes.iter().enumerate() {
        let line_bottom = reference_line_bottom(line, line_advance, size.baseline_bottom);
        stack = Some(stack.unwrap_or(f32::NEG_INFINITY).max(line_bottom));
        if horizontal_ranges_overlap(REF_PAD_X, REF_PAD_X + size.width, label_left, label_right) {
            overlapping_label = Some(
                overlapping_label
                    .unwrap_or(f32::NEG_INFINITY)
                    .max(line_bottom),
            );
        }
    }
    Ok(stack.map(|stack| ReferenceStackBottoms {
        stack,
        overlapping_label,
    }))
}

#[requires(line_advance >= 0.0)]
#[ensures(ret >= 0.0)]
fn reference_line_bottom(line: usize, line_advance: f32, baseline_bottom: f32) -> f32 {
    REF_PAD_Y + TextRole::Reference.font_size() + line as f32 * line_advance + baseline_bottom
}

#[requires(left_start <= left_end)]
#[requires(right_start <= right_end)]
#[ensures(true)]
fn horizontal_ranges_overlap(
    left_start: f32,
    left_end: f32,
    right_start: f32,
    right_end: f32,
) -> bool {
    left_start < right_end && right_start < left_end
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|height| *height > 0.0) || ret.is_err())]
fn gloss_row_height<Tooltip>(
    blocks: &[GentufaBlock<Tooltip>],
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<f32, GentufaExportError> {
    let mut height = GLOSS_ROW_HEIGHT;
    for block in blocks.iter().filter(|block| block.is_leaf) {
        if let Some(gloss) = block_gloss_text(block) {
            let measured = measurer.measure(gloss, TextRole::Gloss, options.script)?;
            height = height.max(measured.height + BLOCK_PADDING * 2.0);
        }
    }
    Ok(height)
}

#[requires(!column_widths.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn grow_columns_for_blocks<Tooltip>(
    column_widths: &mut [f32],
    blocks: &[GentufaBlock<Tooltip>],
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<(), GentufaExportError> {
    let mut sorted = blocks.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|block| block.col_span);
    for block in sorted {
        if block.col >= column_widths.len() {
            continue;
        }
        let required = required_block_width(block, options, measurer)?;
        let span_end = (block.col + block.col_span).min(column_widths.len());
        let current = column_widths[block.col..span_end].iter().sum::<f32>()
            + BLOCK_GAP * span_end.saturating_sub(block.col + 1) as f32;
        if required > current {
            let deficit = required - current;
            let add = deficit / span_end.saturating_sub(block.col).max(1) as f32;
            for width in &mut column_widths[block.col..span_end] {
                *width += add;
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|width| *width > 0.0) || ret.is_err())]
fn required_block_width<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    options: &GentufaSvgOptions,
    measurer: &mut TextMeasurer,
) -> Result<f32, GentufaExportError> {
    let label_role = if block.is_leaf {
        TextRole::LeafLabel
    } else {
        TextRole::NonleafLabel
    };
    let mut width = measure_if_not_empty(measurer, &block.label, label_role, options.script)?
        + BLOCK_PADDING * 2.0;
    let referent_width = markers_width(
        measurer,
        block
            .ref_markers
            .iter()
            .filter(|marker| marker.role == ReferenceMarkerRole::Referent),
        options.script,
    )?;
    width = width.max(referent_width + REF_PAD_X * 2.0);
    let reference_text = reference_source_text(block.ref_markers.iter());
    width = width.max(
        measure_if_not_empty(
            measurer,
            &reference_text,
            TextRole::Reference,
            options.script,
        )? + REF_PAD_X * 2.0,
    );
    if options.show_glosses
        && block.is_leaf
        && let Some(gloss) = block_gloss_text(block)
    {
        width = width.max(
            measure_if_not_empty(measurer, gloss, TextRole::Gloss, options.script)?
                + BLOCK_PADDING * 2.0,
        );
    }
    Ok(width.max(MIN_COLUMN_WIDTH))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|width| *width >= 0.0) || ret.is_err())]
fn measure_if_not_empty(
    measurer: &mut TextMeasurer,
    text: &str,
    role: TextRole,
    script: GentufaScript,
) -> Result<f32, GentufaExportError> {
    if text.is_empty() {
        Ok(0.0)
    } else {
        Ok(measurer.measure(text, role, script)?.width)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|width| *width >= 0.0) || ret.is_err())]
fn markers_width<'a>(
    measurer: &mut TextMeasurer,
    markers: impl Iterator<Item = &'a ReferenceMarker>,
    script: GentufaScript,
) -> Result<f32, GentufaExportError> {
    let mut width: f32 = 0.0;
    for marker in markers {
        width = width.max(
            measurer
                .measure(
                    &reference_label_text(&marker.label),
                    TextRole::Reference,
                    script,
                )?
                .width,
        );
    }
    Ok(width)
}

#[requires(true)]
#[ensures(true)]
fn block_gloss_text<Tooltip>(block: &GentufaBlock<Tooltip>) -> Option<&str> {
    block
        .computed_gloss
        .as_deref()
        .or_else(|| block.glosses.first().map(String::as_str))
        .filter(|text| !text.is_empty())
}

#[requires(true)]
#[ensures(true)]
fn reference_source_text<'a>(markers: impl Iterator<Item = &'a ReferenceMarker>) -> String {
    let mut parts = Vec::new();
    for marker in markers.filter(|marker| marker.role == ReferenceMarkerRole::Reference) {
        parts.push("→".to_owned());
        parts.push(reference_label_text(&marker.label));
    }
    parts.join(" ")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_label_text(label: &ReferenceLabel) -> String {
    let mut output = math_alphanumeric_stem(&label.stem);
    if let Some(occurrence) = label.occurrence {
        output.push_str(&subscript_number(occurrence));
    }
    if let Some(slot) = &label.slot {
        output.push('⟨');
        output.push_str(&reference_slot_display_text(slot));
        output.push('⟩');
    }
    output
}

#[requires(value > 0)]
#[ensures(!ret.is_empty())]
fn subscript_number(value: usize) -> String {
    value.to_string().chars().map(subscript_digit).collect()
}

#[requires(character.is_ascii_digit())]
#[ensures(true)]
fn subscript_digit(character: char) -> char {
    match character {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        _ => unreachable!("requires ASCII digit"),
    }
}

#[requires(true)]
#[ensures(true)]
fn svg_document<Tooltip>(
    layout: &GentufaBlocksLayout<Tooltip>,
    options: &GentufaSvgOptions,
    fonts: GentufaFontData<'_>,
    positioned: &PositionedBlocks,
    measurer: &mut TextMeasurer,
) -> Result<SvgDocument, GentufaExportError> {
    let mut root = SvgElement::new(SvgTag::Svg);
    root.attr("xmlns", SVG_NS);
    root.attr("width", &format_float(positioned.width));
    root.attr("height", &format_float(positioned.height));
    root.attr(
        "viewBox",
        &format!(
            "0 0 {} {}",
            format_float(positioned.width),
            format_float(positioned.height)
        ),
    );
    root.attr("role", "img");
    let mut title = SvgElement::new(SvgTag::Title);
    title.text(&options.title);
    root.child(title);
    let mut style = SvgElement::new(SvgTag::Style);
    style.text(&svg_css(options.script, fonts));
    root.child(style);
    let mut background = SvgElement::new(SvgTag::Rect);
    background.attr("x", "0");
    background.attr("y", "0");
    background.attr("width", &format_float(positioned.width));
    background.attr("height", &format_float(positioned.height));
    background.attr("fill", "#ffffff");
    root.child(background);
    for block in &layout.blocks {
        root.child(block_element(block, options, positioned, measurer)?);
    }
    if options.show_glosses {
        for block in layout.blocks.iter().filter(|block| block.is_leaf) {
            if block_gloss_text(block).is_some() {
                root.child(gloss_element(block, options, positioned));
            }
        }
    }
    Ok(SvgDocument { root })
}

#[requires(true)]
#[ensures(true)]
fn block_element<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    options: &GentufaSvgOptions,
    positioned: &PositionedBlocks,
    measurer: &mut TextMeasurer,
) -> Result<SvgElement, GentufaExportError> {
    let x = positioned.col_x(block.col);
    let y = positioned.row_y(block.row);
    let width = positioned.span_width(block.col, block.col_span);
    let height = positioned.span_height(block.row, block.row_span);
    let mut group = SvgElement::new(SvgTag::G);
    group.attr("id", &block.block_id);
    let mut rect = SvgElement::new(SvgTag::Rect);
    rect.attr("x", &format_float(x));
    rect.attr("y", &format_float(y));
    rect.attr("width", &format_float(width));
    rect.attr("height", &format_float(height));
    rect.attr("fill", &block.color);
    group.child(rect);
    add_referent_text(block, options, &mut group, x, y);
    add_block_label(block, options, &mut group, x, y, width, height);
    add_reference_text(block, options, &mut group, x, y, width, height, measurer)?;
    Ok(group)
}

#[requires(true)]
#[ensures(true)]
fn add_block_label<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    options: &GentufaSvgOptions,
    group: &mut SvgElement,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    if block.label.is_empty() {
        return;
    }
    let role = if block.is_leaf {
        TextRole::LeafLabel
    } else {
        TextRole::NonleafLabel
    };
    let baseline_y = if block.is_leaf {
        y + leaf_label_baseline_y(height)
    } else {
        y + height - NONLEAF_LABEL_BOTTOM_PADDING
    };
    let mut text = text_element(role, options.script, &block.label);
    text.attr("x", &format_float(x + width / 2.0));
    text.attr("y", &format_float(baseline_y));
    text.attr("text-anchor", "middle");
    text.attr("fill", INK);
    if block.is_elided {
        text.attr("text-decoration", "line-through");
        text.attr("text-decoration-thickness", "0.12em");
    }
    group.child(text);
}

#[requires(true)]
#[ensures(true)]
fn add_referent_text<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    options: &GentufaSvgOptions,
    group: &mut SvgElement,
    x: f32,
    y: f32,
) {
    let mut line = 0usize;
    for marker in block
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent)
    {
        let mut text = text_element(
            TextRole::Reference,
            options.script,
            &reference_label_text(&marker.label),
        );
        text.attr("x", &format_float(x + REF_PAD_X));
        text.attr(
            "y",
            &format_float(
                y + REF_PAD_Y
                    + TextRole::Reference.font_size()
                    + line as f32 * (TextRole::Reference.font_size() + REF_LINE_GAP),
            ),
        );
        text.attr("fill", INK);
        group.child(text);
        line += 1;
    }
}

#[requires(true)]
#[ensures(true)]
fn add_reference_text<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    options: &GentufaSvgOptions,
    group: &mut SvgElement,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    measurer: &mut TextMeasurer,
) -> Result<(), GentufaExportError> {
    let text_value = reference_source_text(block.ref_markers.iter());
    if text_value.is_empty() {
        return Ok(());
    }
    let text_size = measurer.measure(&text_value, TextRole::Reference, options.script)?;
    let mut text = text_element(TextRole::Reference, options.script, &text_value);
    text.attr("x", &format_float(x + width - REF_PAD_X));
    text.attr(
        "y",
        &format_float(bottom_aligned_text_baseline_y(
            y + height,
            REF_PAD_Y,
            &text_size,
        )),
    );
    text.attr("text-anchor", "end");
    text.attr("fill", INK);
    group.child(text);
    Ok(())
}

#[requires(block_bottom.is_finite())]
#[requires(bottom_padding.is_finite() && bottom_padding >= 0.0)]
#[ensures(ret <= block_bottom)]
fn bottom_aligned_text_baseline_y(
    block_bottom: f32,
    bottom_padding: f32,
    text_size: &TextSize,
) -> f32 {
    block_bottom - bottom_padding - text_size.baseline_bottom
}

#[requires(true)]
#[ensures(true)]
fn gloss_element<Tooltip>(
    block: &GentufaBlock<Tooltip>,
    options: &GentufaSvgOptions,
    positioned: &PositionedBlocks,
) -> SvgElement {
    let x = positioned.col_x(block.col);
    let y = positioned.gloss_y();
    let width = positioned.span_width(block.col, block.col_span);
    let height = positioned.gloss_row_height.unwrap_or(GLOSS_ROW_HEIGHT);
    let mut group = SvgElement::new(SvgTag::G);
    let mut rect = SvgElement::new(SvgTag::Rect);
    rect.attr("x", &format_float(x));
    rect.attr("y", &format_float(y));
    rect.attr("width", &format_float(width));
    rect.attr("height", &format_float(height));
    rect.attr("fill", GLOSS_BG);
    group.child(rect);
    if let Some(gloss) = block_gloss_text(block) {
        let mut text = text_element(TextRole::Gloss, options.script, gloss);
        text.attr("x", &format_float(x + BLOCK_PADDING));
        text.attr(
            "y",
            &format_float(y + BLOCK_PADDING + TextRole::Gloss.font_size()),
        );
        text.attr("fill", GLOSS_INK);
        group.child(text);
    }
    group
}

#[requires(!value.is_empty())]
#[ensures(true)]
fn text_element(role: TextRole, script: GentufaScript, value: &str) -> SvgElement {
    let mut text = SvgElement::new(SvgTag::Text);
    text.attr("font-family", role.font_family(script));
    text.attr("font-size", &format_float(role.font_size()));
    text.attr("font-weight", role.font_weight());
    text.attr("font-style", role.font_style());
    text.attr("dominant-baseline", "alphabetic");
    text.text(value);
    text
}

#[requires(true)]
#[ensures(ret.contains("@font-face"))]
fn svg_css(script: GentufaScript, fonts: GentufaFontData<'_>) -> String {
    let crisa = if script == GentufaScript::Zbalermorna {
        crisa_font_face(fonts.crisa)
    } else {
        String::new()
    };
    format!(
        r#"
@font-face {{
  font-family: "Noto Sans";
  src: url("https://cdn.jsdelivr.net/fontsource/fonts/noto-sans:vf@latest/latin-wght-normal.woff2") format("woff2-variations");
  font-weight: 100 900;
  font-style: normal;
}}
@font-face {{
  font-family: "Noto Sans";
  src: url("https://cdn.jsdelivr.net/fontsource/fonts/noto-sans:vf@latest/latin-wght-italic.woff2") format("woff2-variations");
  font-weight: 100 900;
  font-style: italic;
}}
@font-face {{
  font-family: "STIX Two Math";
  src: url("https://fonts.gstatic.com/s/stixtwomath/v12/pONg1hwwL_6M9EkZySr_yteUi1o.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
}}
@font-face {{
  font-family: "STIX Two Text";
  src: url("https://fonts.gstatic.com/s/stixtwotext/v18/YA9Gr02F12Xkf5whdwKf11l0jbKkeidMTtZ5Yihg2SOY.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
}}
@font-face {{
  font-family: "STIX Two Text";
  src: url("https://fonts.gstatic.com/s/stixtwotext/v18/YA9Gr02F12Xkf5whdwKf11l0jbKkeidMTtZ5YiiH3iOY.ttf") format("truetype");
  font-weight: 700;
  font-style: normal;
}}{}"#,
        crisa
    )
}

#[requires(true)]
#[ensures(ret.is_empty() || ret.contains("Crisa"))]
fn crisa_font_face(font: Option<&[u8]>) -> String {
    if let Some(font) = font {
        let encoded = base64::engine::general_purpose::STANDARD.encode(font);
        format!(
            r#"
@font-face {{
  font-family: "Crisa";
  src: url("data:font/otf;base64,{encoded}") format("opentype");
  font-weight: 400;
  font-style: normal;
}}"#
        )
    } else {
        r#"
@font-face {
  font-family: "Crisa";
  src: local("Crisa");
  font-weight: 400;
  font-style: normal;
}"#
        .to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn usvg_options(fonts: GentufaFontData<'_>) -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options
        .fontdb_mut()
        .load_font_data(fonts.noto_sans.to_vec());
    options
        .fontdb_mut()
        .load_font_data(fonts.noto_sans_italic.to_vec());
    options
        .fontdb_mut()
        .load_font_data(fonts.stix_two_math.to_vec());
    options
        .fontdb_mut()
        .load_font_data(fonts.stix_two_text.to_vec());
    options
        .fontdb_mut()
        .load_font_data(fonts.stix_two_text_bold.to_vec());
    if let Some(crisa) = fonts.crisa {
        options.fontdb_mut().load_font_data(crisa.to_vec());
    }
    options.fontdb_mut().set_sans_serif_family("Noto Sans");
    options.font_family = "Noto Sans".to_owned();
    options
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SvgDocument {
    root: SvgElement,
}

impl SvgDocument {
    #[requires(true)]
    #[ensures(ret.starts_with("<svg"))]
    fn to_xml(&self) -> String {
        let mut writer = XmlWriter::new(XmlOptions {
            indent: Indent::None,
            ..XmlOptions::default()
        });
        self.root.write(&mut writer);
        writer.end_document()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SvgElement {
    tag: SvgTag,
    attributes: Vec<SvgAttribute>,
    children: Vec<SvgNode>,
}

impl SvgElement {
    #[requires(true)]
    #[ensures(ret.tag == tag)]
    fn new(tag: SvgTag) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    #[requires(!name.is_empty())]
    #[ensures(true)]
    fn attr(&mut self, name: &'static str, value: &str) {
        self.attributes.push(SvgAttribute {
            name,
            value: value.to_owned(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn child(&mut self, element: SvgElement) {
        self.children.push(SvgNode::Element(element));
    }

    #[requires(true)]
    #[ensures(true)]
    fn text(&mut self, text: &str) {
        self.children.push(SvgNode::Text(text.to_owned()));
    }

    #[requires(true)]
    #[ensures(true)]
    fn write(&self, writer: &mut XmlWriter) {
        writer.start_element(self.tag.name());
        for attribute in &self.attributes {
            writer.write_attribute(attribute.name, &escape_xml_attribute(&attribute.value));
        }
        let preserve = self.tag == SvgTag::Style || self.tag == SvgTag::Text;
        if preserve {
            writer.set_preserve_whitespaces(true);
        }
        for child in &self.children {
            child.write(writer);
        }
        if preserve {
            writer.set_preserve_whitespaces(false);
        }
        writer.end_element();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum SvgTag {
    Svg,
    Style,
    Rect,
    Text,
    G,
    Title,
}

impl SvgTag {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn name(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Style => "style",
            Self::Rect => "rect",
            Self::Text => "text",
            Self::G => "g",
            Self::Title => "title",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SvgAttribute {
    name: &'static str,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Element(_) => true)]
#[invariant(::Text(_) => true)]
enum SvgNode {
    Element(SvgElement),
    Text(String),
}

impl SvgNode {
    #[requires(true)]
    #[ensures(true)]
    fn write(&self, writer: &mut XmlWriter) {
        match self {
            Self::Element(element) => element.write(writer),
            Self::Text(text) => writer.write_text(&escape_xml_text(text)),
        }
    }
}

#[requires(true)]
#[ensures(!ret.contains('<'))]
#[ensures(!ret.contains('&') || ret.contains("&amp;") || ret.contains("&lt;") || ret.contains("&gt;"))]
fn escape_xml_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
    output
}

#[requires(true)]
#[ensures(!ret.contains('<'))]
#[ensures(!ret.contains('"'))]
fn escape_xml_attribute(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
    output
}

#[requires(value.is_finite())]
#[ensures(!ret.is_empty())]
fn format_float(value: f32) -> String {
    let mut text = format!("{value:.2}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn svg_serialization_escapes_text() {
        let mut root = SvgElement::new(SvgTag::Svg);
        root.attr("xmlns", SVG_NS);
        root.attr("width", "120");
        root.attr("height", "40");
        root.attr("viewBox", "0 0 120 40");
        let mut text = SvgElement::new(SvgTag::Text);
        text.attr("x", "0");
        text.attr("y", "20");
        text.attr("data-test", "a&\"<b");
        text.text("mi <do> & ko");
        root.child(text);
        let svg = SvgDocument { root }.to_xml();
        assert!(svg.contains("data-test=\"a&amp;&quot;&lt;b\""));
        assert!(svg.contains("&lt;do&gt;"));
        assert!(svg.contains("&amp;"));
        let xml = roxmltree::Document::parse(&svg).expect("generated XML parses");
        let _tree = usvg::Tree::from_xmltree(&xml, &usvg_options(EmbeddedGentufaFonts::get()))
            .expect("generated SVG parses");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn text_measurement_returns_nonzero_size() {
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let size = measurer
            .measure("mi klama", TextRole::LeafLabel, GentufaScript::Latin)
            .expect("measurement");
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
        let math_size = measurer
            .measure("𝑘₁⟨𝟣⟩", TextRole::Reference, GentufaScript::Latin)
            .expect("math reference measurement");
        assert!(math_size.width > 0.0);
        assert!(math_size.height > 0.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn svg_reference_labels_use_font_cascade_and_styled_slot_text() {
        let mut block = test_gentufa_block(0, 1, 0);
        block.ref_markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Referent,
            kind: "argument".to_owned(),
            label: ReferenceLabel::new(
                "b",
                Some(2),
                Some(ReferenceSlotLabel::Modal(vec![
                    "mléca".to_owned(),
                    "be\u{301}rvi".to_owned(),
                ])),
            ),
        });
        let layout = GentufaBlocksLayout {
            blocks: vec![block],
            max_col: 1,
            max_row: 1,
        };

        let svg = render_gentufa_blocks_svg(
            &layout,
            &GentufaSvgOptions::default(),
            EmbeddedGentufaFonts::get(),
        )
        .expect("svg");

        assert!(svg.contains("font-family=\"STIX Two Text, STIX Two Math, serif\""));
        assert!(svg.contains("font-family: \"STIX Two Math\""));
        assert!(svg.contains("font-family: \"STIX Two Text\""));
        assert!(svg.contains("𝑏₂⟨𝗆𝗅𝖾𝖼𝖺 𝖻𝖾𝗋𝗏𝗂⟩"));
        assert!(!svg.contains("mleca bervi"));
        assert!(!svg.contains("mléca"));
        assert!(!svg.contains('\u{0301}'));
        let xml = roxmltree::Document::parse(&svg).expect("generated XML parses");
        let _tree = usvg::Tree::from_xmltree(&xml, &usvg_options(EmbeddedGentufaFonts::get()))
            .expect("generated SVG parses");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn svg_reference_labels_style_numbered_slot_text_only() {
        let mut block = test_gentufa_block(0, 1, 0);
        block.ref_markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Referent,
            kind: "argument".to_owned(),
            label: ReferenceLabel::new("b", Some(2), Some(ReferenceSlotLabel::Numbered(1))),
        });
        let layout = GentufaBlocksLayout {
            blocks: vec![block],
            max_col: 1,
            max_row: 1,
        };

        let svg = render_gentufa_blocks_svg(
            &layout,
            &GentufaSvgOptions::default(),
            EmbeddedGentufaFonts::get(),
        )
        .expect("svg");

        assert!(svg.contains("𝑏₂⟨𝟣⟩"));
        assert!(!svg.contains("𝑏₂⟨1⟩"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leaf_label_height_reserves_top_padding_without_references() {
        let options = GentufaSvgOptions::default();
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let mut row_heights = vec![ROW_COMPACT_HEIGHT];
        let blocks = vec![test_gentufa_block(0, 1, 0)];

        grow_rows_for_leaf_labels(&mut row_heights, &blocks, &options, &mut measurer)
            .expect("row growth");

        assert!(row_heights[0] > ROW_COMPACT_HEIGHT);
        let label_size = measurer
            .measure(&blocks[0].label, TextRole::LeafLabel, options.script)
            .expect("label measurement");
        let label_top = leaf_label_baseline_y(row_heights[0]) + label_size.baseline_top;
        assert!(label_top >= BLOCK_PADDING - 0.01);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn outgoing_reference_baseline_keeps_ink_inside_bottom_padding() {
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let text_size = measurer
            .measure("→ 𝑏₃", TextRole::Reference, GentufaScript::Latin)
            .expect("reference measurement");

        let baseline = bottom_aligned_text_baseline_y(ROW_COMPACT_HEIGHT, REF_PAD_Y, &text_size);

        assert!(baseline < ROW_COMPACT_HEIGHT - REF_PAD_Y);
        assert!(baseline + text_size.baseline_bottom <= ROW_COMPACT_HEIGHT - REF_PAD_Y + 0.01);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn single_incoming_reference_grows_single_row_leaf() {
        let options = GentufaSvgOptions::default();
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let mut row_heights = vec![ROW_COMPACT_HEIGHT];
        let column_widths = vec![MIN_COLUMN_WIDTH];
        let mut block = test_gentufa_block(0, 1, 0);
        block.ref_markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Referent,
            kind: "test".to_owned(),
            label: ReferenceLabel::new(
                "b",
                Some(1),
                Some(ReferenceSlotLabel::Modal(vec![
                    "mleca".to_owned(),
                    "bervi".to_owned(),
                ])),
            ),
        });
        let blocks = vec![block];

        grow_rows_for_references(
            &mut row_heights,
            &column_widths,
            &blocks,
            &options,
            &mut measurer,
        )
        .expect("row growth");

        assert!(row_heights[0] > ROW_COMPACT_HEIGHT);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn single_incoming_reference_does_not_grow_spanning_leaf() {
        let options = GentufaSvgOptions::default();
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let mut row_heights = vec![ROW_COMPACT_HEIGHT; 3];
        let column_widths = vec![MIN_COLUMN_WIDTH];
        let blocks = vec![test_gentufa_block(0, 3, 1)];

        grow_rows_for_references(
            &mut row_heights,
            &column_widths,
            &blocks,
            &options,
            &mut measurer,
        )
        .expect("row growth");

        assert_eq!(row_heights, vec![ROW_COMPACT_HEIGHT; 3]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn horizontally_separate_incoming_references_do_not_grow_nonleaf_row() {
        let options = GentufaSvgOptions::default();
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let mut row_heights = vec![ROW_COMPACT_HEIGHT];
        let column_widths = vec![MIN_COLUMN_WIDTH; 3];
        let blocks = vec![test_wide_nonleaf_block(0, 1, 3, 2)];

        grow_rows_for_references(
            &mut row_heights,
            &column_widths,
            &blocks,
            &options,
            &mut measurer,
        )
        .expect("row growth");

        assert_eq!(row_heights, vec![ROW_COMPACT_HEIGHT]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn horizontally_separate_incoming_references_grow_to_stay_inside_nonleaf_block() {
        let options = GentufaSvgOptions::default();
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let mut row_heights = vec![ROW_COMPACT_HEIGHT];
        let column_widths = vec![MIN_COLUMN_WIDTH; 3];
        let blocks = vec![test_wide_nonleaf_block(0, 1, 3, 3)];

        grow_rows_for_references(
            &mut row_heights,
            &column_widths,
            &blocks,
            &options,
            &mut measurer,
        )
        .expect("row growth");

        assert!(row_heights[0] > ROW_COMPACT_HEIGHT);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn many_incoming_references_grow_only_spanning_leaf_bottom_row() {
        let options = GentufaSvgOptions::default();
        let mut measurer = TextMeasurer::new(EmbeddedGentufaFonts::get());
        let mut row_heights = vec![ROW_COMPACT_HEIGHT; 3];
        let column_widths = vec![MIN_COLUMN_WIDTH];
        let blocks = vec![test_gentufa_block(0, 3, 12)];

        grow_rows_for_references(
            &mut row_heights,
            &column_widths,
            &blocks,
            &options,
            &mut measurer,
        )
        .expect("row growth");

        assert_eq!(row_heights[0], ROW_COMPACT_HEIGHT);
        assert_eq!(row_heights[1], ROW_COMPACT_HEIGHT);
        assert!(row_heights[2] > ROW_COMPACT_HEIGHT);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn png_render_has_magic_header() {
        let layout = GentufaBlocksLayout {
            blocks: vec![GentufaBlock {
                block_id: "n1".to_owned(),
                node_ids: vec![1],
                label: "mi".to_owned(),
                is_leaf: true,
                is_elided: false,
                token_kind: Some("word".to_owned()),
                ref_markers: Vec::new(),
                span: None,
                node_types: Vec::new(),
                ancestors: Vec::new(),
                col: 0,
                col_span: 1,
                row: 0,
                row_span: 1,
                color: "#ffffff".to_owned(),
                parent_color: None,
                raw_text: "mi".to_owned(),
                display_text: "mi".to_owned(),
                transform: None,
                glosses: Vec::new(),
                definition: None,
                computed_gloss: None,
                tooltip: None::<()>,
            }],
            max_col: 1,
            max_row: 1,
        };
        let fonts = EmbeddedGentufaFonts::get();
        let svg =
            render_gentufa_blocks_svg(&layout, &GentufaSvgOptions::default(), fonts).expect("svg");
        let xml = roxmltree::Document::parse(&svg).expect("generated SVG XML");
        let svg_root = xml.root_element();
        let svg_width = svg_root
            .attribute("width")
            .expect("SVG width")
            .parse::<f32>()
            .expect("SVG width number");
        let svg_height = svg_root
            .attribute("height")
            .expect("SVG height")
            .parse::<f32>()
            .expect("SVG height number");
        let png =
            render_gentufa_blocks_png(&layout, &GentufaPngOptions::default(), fonts).expect("png");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        let width = u32::from_be_bytes(png[16..20].try_into().expect("png width bytes"));
        let height = u32::from_be_bytes(png[20..24].try_into().expect("png height bytes"));
        assert_eq!(width, (svg_width * DEFAULT_GENTUFA_PNG_SCALE).ceil() as u32);
        assert_eq!(
            height,
            (svg_height * DEFAULT_GENTUFA_PNG_SCALE).ceil() as u32
        );
    }

    #[requires(row_span > 0)]
    #[ensures(ret.row == row)]
    fn test_gentufa_block(row: usize, row_span: usize, incoming_count: usize) -> GentufaBlock<()> {
        GentufaBlock {
            block_id: format!("test-{row}"),
            node_ids: Vec::new(),
            label: "ny".to_owned(),
            is_leaf: true,
            is_elided: false,
            token_kind: Some("cmavo".to_owned()),
            ref_markers: (0..incoming_count).map(test_reference_marker).collect(),
            span: None,
            node_types: Vec::new(),
            ancestors: Vec::new(),
            col: 0,
            col_span: 1,
            row,
            row_span,
            color: "#ffffff".to_owned(),
            parent_color: None,
            raw_text: "ny".to_owned(),
            display_text: "ny".to_owned(),
            transform: None,
            glosses: Vec::new(),
            definition: None,
            computed_gloss: None,
            tooltip: None,
        }
    }

    #[requires(row_span > 0)]
    #[requires(col_span > 0)]
    #[ensures(ret.row == row)]
    fn test_wide_nonleaf_block(
        row: usize,
        row_span: usize,
        col_span: usize,
        incoming_count: usize,
    ) -> GentufaBlock<()> {
        GentufaBlock {
            block_id: format!("wide-{row}"),
            node_ids: Vec::new(),
            label: "Cei".to_owned(),
            is_leaf: false,
            is_elided: false,
            token_kind: None,
            ref_markers: (0..incoming_count).map(test_reference_marker).collect(),
            span: None,
            node_types: Vec::new(),
            ancestors: Vec::new(),
            col: 0,
            col_span,
            row,
            row_span,
            color: "#ffffff".to_owned(),
            parent_color: None,
            raw_text: "cei".to_owned(),
            display_text: String::new(),
            transform: None,
            glosses: Vec::new(),
            definition: None,
            computed_gloss: None,
            tooltip: None,
        }
    }

    #[requires(true)]
    #[ensures(ret.role == ReferenceMarkerRole::Referent)]
    fn test_reference_marker(index: usize) -> ReferenceMarker {
        ReferenceMarker {
            role: ReferenceMarkerRole::Referent,
            kind: "test".to_owned(),
            label: ReferenceLabel::new("b", Some(index + 1), None),
        }
    }
}

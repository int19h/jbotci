//! The Discord diagram policy for gentufa block PNGs.
//!
//! One render policy, no fallbacks: the shared SVG renderer lays the diagram
//! out at the shared scale, the pixel area is checked *before* any raster
//! buffer is allocated, and the encoded bytes are checked against both the
//! application cap and the per-interaction `attachment_size_limit` Discord
//! reports. A diagram outside the limits is refused with a clear reason; the
//! caller keeps the previous coherent state. The numeric limits are what the
//! deployed instance was measured to afford beside its analysis lane and its
//! embedding model; `docs/discord-app.md` records the measurement and the
//! reasoning behind each figure.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_web_core::{
    DEFAULT_GENTUFA_PNG_SCALE, GentufaBlocksLayout, GentufaExportFormat, GentufaScript,
    render_gentufa_blocks_web_export,
};

/// Bounds applied to every Discord diagram.
#[invariant(*max_blocks > 0 && *max_columns > 0 && *max_pixels > 0 && *max_bytes > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiagramLimits {
    /// Layout blocks (nodes and leaves) the renderer will lay out.
    pub(crate) max_blocks: usize,
    /// Grid columns (leaves) across.
    pub(crate) max_columns: usize,
    /// Rasterized pixel area (width × height after scaling).
    pub(crate) max_pixels: u64,
    /// Encoded PNG bytes.
    pub(crate) max_bytes: usize,
}

impl Default for DiagramLimits {
    #[requires(true)]
    #[ensures(ret.max_blocks > 0)]
    fn default() -> Self {
        new!(DiagramLimits {
            max_blocks: 600,
            max_columns: 160,
            // 8 megapixels of RGBA is a 32 MiB raster buffer. Measured, one
            // render near that size costs the process about 50 MiB including
            // the layout, the encoder and what the allocator keeps; the
            // instance has 512 MiB and the embedding model takes about 240 of
            // them, so this is what one image may spend.
            max_pixels: 8_000_000,
            max_bytes: 8 * 1024 * 1024,
        })
    }
}

/// A rendered diagram ready to attach.
#[invariant(!bytes.is_empty() && *width > 0 && *height > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagramImage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

#[invariant(::TooComplex { .. } => true)]
#[invariant(::TooLarge { .. } => true)]
#[invariant(::TooManyBytes { .. } => true)]
#[invariant(::Render { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiagramError {
    TooComplex { blocks: usize, columns: usize },
    TooLarge { width: usize, height: usize },
    TooManyBytes { bytes: usize, limit: usize },
    Render { message: String },
}

impl fmt::Display for DiagramError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooComplex { blocks, columns } => write!(
                formatter,
                "the diagram is too complex for a Discord image ({blocks} blocks across {columns} columns)"
            ),
            Self::TooLarge { width, height } => write!(
                formatter,
                "the diagram would be {width}×{height} pixels, larger than a Discord image may be"
            ),
            Self::TooManyBytes { bytes, limit } => write!(
                formatter,
                "the diagram encodes to {bytes} bytes, over the {limit}-byte attachment limit"
            ),
            Self::Render { message } => {
                write!(formatter, "the diagram could not be rendered ({message})")
            }
        }
    }
}

impl std::error::Error for DiagramError {}

/// Render the diagram PNG under `limits`. `attachment_size_limit` is the
/// per-interaction byte cap Discord reported, when known.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|image| image.bytes.len() <= limits.max_bytes) || ret.is_err())]
pub(crate) fn render_diagram(
    layout: &GentufaBlocksLayout,
    show_glosses: bool,
    limits: DiagramLimits,
    attachment_size_limit: Option<u64>,
) -> Result<DiagramImage, DiagramError> {
    let blocks = layout.blocks.len();
    let columns = layout.max_col;
    if blocks > limits.max_blocks || columns > limits.max_columns {
        return Err(DiagramError::TooComplex { blocks, columns });
    }
    // Lay out as SVG first: this is text measurement only, no raster
    // allocation, and it yields the dimensions the PNG pass would rasterize.
    let svg = render_gentufa_blocks_web_export(
        layout,
        show_glosses,
        GentufaScript::Latin,
        GentufaExportFormat::Svg,
    )
    .map_err(|error| DiagramError::Render {
        message: error.to_string(),
    })?;
    let (svg_width, svg_height) = match (svg.width, svg.height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => (width, height),
        _ => {
            return Err(DiagramError::Render {
                message: "the diagram layout has no size".to_owned(),
            });
        }
    };
    let scale = f64::from(DEFAULT_GENTUFA_PNG_SCALE);
    let width = (svg_width as f64 * scale).ceil() as usize;
    let height = (svg_height as f64 * scale).ceil() as usize;
    if (width as u64).saturating_mul(height as u64) > limits.max_pixels {
        return Err(DiagramError::TooLarge { width, height });
    }
    let png = render_gentufa_blocks_web_export(
        layout,
        show_glosses,
        GentufaScript::Latin,
        GentufaExportFormat::Png,
    )
    .map_err(|error| DiagramError::Render {
        message: error.to_string(),
    })?;
    let byte_limit = attachment_size_limit
        .and_then(|limit| usize::try_from(limit).ok())
        .map_or(limits.max_bytes, |limit| limit.min(limits.max_bytes));
    if png.bytes.len() > byte_limit {
        return Err(DiagramError::TooManyBytes {
            bytes: png.bytes.len(),
            limit: byte_limit,
        });
    }
    if png.bytes.is_empty() {
        return Err(DiagramError::Render {
            message: "the PNG encoder produced no bytes".to_owned(),
        });
    }
    Ok(new!(DiagramImage {
        bytes: png.bytes,
        width: png.width.unwrap_or(width),
        height: png.height.unwrap_or(height),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_web_core::{
        GentufaWebOptions, GentufaWebRequest, GentufaWebResult, parse_gentufa_for_web,
    };

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn layout(text: &str) -> GentufaBlocksLayout {
        let GentufaWebResult::Success(success) = parse_gentufa_for_web(&GentufaWebRequest {
            text: text.to_owned(),
            options: GentufaWebOptions::default(),
        }) else {
            panic!("{text} parses");
        };
        success.blocks_layout
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_a_small_diagram_within_default_limits() {
        let image = render_diagram(&layout("mi klama"), false, DiagramLimits::default(), None)
            .expect("diagram");
        assert!(image.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(image.width > 0 && image.height > 0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn refuses_complexity_area_and_byte_overflow_before_committing() {
        let layout = layout("mi klama lo zarci");
        let tiny_blocks = DiagramLimits::default().with_data(bityzba::data! { max_blocks: 1 });
        assert!(matches!(
            render_diagram(&layout, false, tiny_blocks, None),
            Err(DiagramError::TooComplex { .. })
        ));
        let tiny_area = DiagramLimits::default().with_data(bityzba::data! { max_pixels: 1 });
        assert!(matches!(
            render_diagram(&layout, false, tiny_area, None),
            Err(DiagramError::TooLarge { .. })
        ));
        let tiny_bytes = DiagramLimits::default().with_data(bityzba::data! { max_bytes: 1 });
        assert!(matches!(
            render_diagram(&layout, false, tiny_bytes, None),
            Err(DiagramError::TooManyBytes { limit: 1, .. })
        ));
        // The interaction's own attachment limit wins when it is smaller.
        assert!(matches!(
            render_diagram(&layout, false, DiagramLimits::default(), Some(16)),
            Err(DiagramError::TooManyBytes { limit: 16, .. })
        ));
    }
}

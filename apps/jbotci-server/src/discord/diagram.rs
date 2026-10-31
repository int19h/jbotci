//! The Discord diagram policy for gentufa block PNGs.
//!
//! One render policy, no fallbacks: the shared SVG renderer lays the diagram
//! out at the shared scale, the pixel area is checked *before* any raster
//! buffer is allocated, and the encoded bytes are checked against both the
//! application cap and the per-interaction `attachment_size_limit` Discord
//! reports. A diagram outside the limits is refused with a clear reason; the
//! caller keeps the previous coherent state. The numeric limits are informed
//! by local workload measurements against the service memory budget;
//! `docs/discord-app.md` records the workload, architecture limits, and
//! reasoning behind each figure.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_web_core::{
    DEFAULT_GENTUFA_PNG_SCALE, GentufaBlocksLayout, GentufaExportFormat, GentufaScript,
    render_gentufa_blocks_web_export_with_limits,
};

pub(crate) use crate::policy::DiagramLimits;

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
            Self::TooManyBytes { limit, .. } => write!(
                formatter,
                "the diagram exceeds the {limit}-byte attachment limit"
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
    let svg = render_gentufa_blocks_web_export_with_limits(
        layout,
        show_glosses,
        GentufaScript::Latin,
        GentufaExportFormat::Svg,
        Some(limits),
    )
    .map_err(diagram_export_error)?;
    let svg_text = std::str::from_utf8(&svg.bytes).expect("SVG export writes UTF-8");
    let plan = jbotci_gentufa::checked_svg_raster_plan(
        svg_text,
        DEFAULT_GENTUFA_PNG_SCALE,
        limits.max_pixels,
    )
    .map_err(|error| DiagramError::Render {
        message: error.to_string(),
    })?;
    let width = plan.width as usize;
    let height = plan.height as usize;
    let byte_limit = attachment_size_limit
        .and_then(|limit| usize::try_from(limit).ok())
        .map_or(limits.max_bytes, |limit| limit.min(limits.max_bytes));
    if byte_limit == 0 {
        return Err(DiagramError::TooManyBytes { bytes: 1, limit: 0 });
    }
    let png = render_gentufa_blocks_web_export_with_limits(
        layout,
        show_glosses,
        GentufaScript::Latin,
        GentufaExportFormat::Png,
        Some(limits.with_data(bityzba::data! { max_bytes: byte_limit })),
    )
    .map_err(diagram_export_error)?;
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
        let image = render_diagram(
            &layout("mi klama"),
            false,
            crate::policy::default_diagram_limits(),
            None,
        )
        .expect("diagram");
        assert!(image.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(image.width > 0 && image.height > 0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn refuses_complexity_area_and_byte_overflow_before_committing() {
        let layout = layout("mi klama lo zarci");
        let tiny_blocks =
            crate::policy::default_diagram_limits().with_data(bityzba::data! { max_blocks: 1 });
        assert!(matches!(
            render_diagram(&layout, false, tiny_blocks, None),
            Err(DiagramError::TooComplex { .. })
        ));
        let tiny_area =
            crate::policy::default_diagram_limits().with_data(bityzba::data! { max_pixels: 1 });
        assert!(matches!(
            render_diagram(&layout, false, tiny_area, None),
            Err(DiagramError::TooLarge { .. })
        ));
        let tiny_bytes =
            crate::policy::default_diagram_limits().with_data(bityzba::data! { max_bytes: 1 });
        assert!(matches!(
            render_diagram(&layout, false, tiny_bytes, None),
            Err(DiagramError::TooManyBytes { limit: 1, .. })
        ));
        // The interaction's own attachment limit wins when it is smaller.
        assert!(matches!(
            render_diagram(
                &layout,
                false,
                crate::policy::default_diagram_limits(),
                Some(16)
            ),
            Err(DiagramError::TooManyBytes { limit: 16, .. })
        ));
    }
}

#[requires(true)]
#[ensures(true)]
fn diagram_export_error(error: jbotci_web_core::WebComputeError) -> DiagramError {
    match error.diagram_limit().as_ref().map(|limit| limit.as_data()) {
        Some(data!(jbotci_gentufa::GentufaLimit::Layout { blocks, columns })) => {
            DiagramError::TooComplex {
                blocks: *blocks,
                columns: *columns,
            }
        }
        Some(data!(jbotci_gentufa::GentufaLimit::Pixels { width, height })) => {
            DiagramError::TooLarge {
                width: *width as usize,
                height: *height as usize,
            }
        }
        Some(data!(jbotci_gentufa::GentufaLimit::Bytes { limit })) => DiagramError::TooManyBytes {
            bytes: limit.get().saturating_add(1),
            limit: limit.get(),
        },
        None => DiagramError::Render {
            message: error.to_string(),
        },
    }
}

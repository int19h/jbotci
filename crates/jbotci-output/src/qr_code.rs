//! QR encoding and logo rendering for jo'au dialect payloads.
//!
//! This module intentionally implements only ISO/IEC 18004 Model 2 QR codes in
//! alphanumeric mode with level-H error correction, which is the only shape used
//! for jo'au dialect payloads in the UI.

use std::collections::BTreeSet;
use std::fmt::Write;

#[allow(unused_imports)]
use bityzba::ensures;
#[allow(unused_imports)]
use bityzba::expensive_invariant;
use bityzba::{invariant, new, requires};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
pub struct QrCoord {
    pub x: i32,
    pub y: i32,
}

#[invariant(*version >= 1 && *version <= 40, "QR version must be in the standard range")]
#[invariant(*size == *version * 4 + 17, "QR size must match the version")]
#[invariant(*mask >= 0 && *mask <= 7, "QR mask must be one of the eight standard masks")]
#[expensive_invariant(
    dark_modules.iter().all(|coord| coord.x >= 0 && coord.y >= 0 && coord.x < *size && coord.y < *size),
    "QR dark modules must be inside the matrix"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrCode {
    pub version: i32,
    pub size: i32,
    pub mask: i32,
    pub dark_modules: BTreeSet<QrCoord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct QrBuild {
    dark_modules: BTreeSet<QrCoord>,
    function_modules: BTreeSet<QrCoord>,
}

#[invariant(!data_codewords.is_empty(), "QR data blocks must contain data")]
#[invariant(!error_codewords.is_empty(), "QR data blocks must contain ECC")]
#[expensive_invariant(
    data_codewords.iter().all(|codeword| (0..=255).contains(codeword)),
    "QR data codewords must be bytes"
)]
#[expensive_invariant(
    error_codewords.iter().all(|codeword| (0..=255).contains(codeword)),
    "QR error-correction codewords must be bytes"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct QrBlock {
    data_codewords: Vec<i32>,
    error_codewords: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
struct QrLogoLayer {
    color: &'static str,
    path_data: &'static str,
    translate_x: f64,
    translate_y: f64,
    scale: f64,
}

#[invariant(*left >= 0, "QR logo placement left edge must be inside the matrix")]
#[invariant(*top >= 0, "QR logo placement top edge must be inside the matrix")]
#[invariant(
    *side >= QR_LOGO_MINIMUM_SIDE,
    "QR logo placement side length must satisfy the minimum logo size"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QrLogoPlacement {
    left: i32,
    top: i32,
    side: i32,
}

#[invariant(*version >= 1 && *version <= 40, "QR logo layout version must be valid")]
#[invariant(*size_value == qr_size(*version), "QR logo layout size must match version")]
#[invariant(*correction_budget >= 0, "QR logo correction budget must not be negative")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct QrLogoLayout {
    version: i32,
    size_value: i32,
    function_modules: BTreeSet<QrCoord>,
    data_placements: Vec<QrCoord>,
    codeword_blocks: Vec<usize>,
    correction_budget: i32,
}

const QR_LOGO_BADGE_BACKGROUND: &str = "#cbd4ff";
const QR_LOGO_BADGE_SIZE: f64 = 7.0;
const QR_LOGO_BORDER_WIDTH: f64 = 0.25;
const QR_LOGO_INSET: f64 = 1.0;
const QR_LOGO_BASE_CONTENT_SIZE: f64 = QR_LOGO_BADGE_SIZE - QR_LOGO_INSET * 2.0;

const QR_LOGO_MINIMUM_SIDE: i32 = 7;
const QR_LOGO_FUNCTION_CLEARANCE: i32 = 1;
const QR_LOGO_ERROR_CORRECTION_MARGIN: i32 = 2;
const QR_LOGO_RED: &str = "#f45f86";
const QR_LOGO_BLUE: &str = "#466cff";
const ALPHANUMERIC_CHARACTERS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

// ISO/IEC 18004:2015, Annex E, error correction characteristics for level H.
const HIGH_ECC_CODEWORDS_PER_BLOCK: [i32; 41] = [
    0, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 28, 28, 28,
    30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
];

// ISO/IEC 18004:2015, Annex E, number of error correction blocks for level H.
const HIGH_ERROR_CORRECTION_BLOCK_COUNT: [i32; 41] = [
    0, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35,
    37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81,
];

#[requires(true)]
#[ensures(!ret.is_empty())]
fn qr_logo_layers() -> Vec<QrLogoLayer> {
    vec![
        QrLogoLayer {
            color: QR_LOGO_RED,
            path_data: "M404 73L411 18C414 6 421 0 434 0L487 0L487 500L484 500L398 500L398 132C379 111 358 94 335 81C313 68 289 62 263 62C228 62 201 73 184 94C172 109 163 138 158 182L158 500L75 500L70 500L70 -250L137 -250L158 -250L158 48L161 44C172 28 186 15 204 6C222 -3 242 -8 264 -8C279 -8 293 -6 306 -2C319 2 331 7 342 14C354 21 364 30 374 40C375 41 386 52 404 73Z",
            translate_x: 0.530754,
            translate_y: 3.759921,
            scale: 0.00496032,
        },
        QrLogoLayer {
            color: QR_LOGO_BLUE,
            path_data: "M30 651C24 651 18 653 13 658C12 659 6 666 -5 679C-16 689 -30 702 -49 702C-61 702 -73 697 -84 689C-113 673 -127 642 -127 595L-78 595C-77 627 -67 644 -47 644C-42 644 -36 642 -30 637C-13 623 3 593 33 593C46 593 57 596 68 601C99 617 115 650 115 699L67 699C66 667 54 651 30 651Z",
            translate_x: 1.889881,
            translate_y: 3.75496,
            scale: 0.00496032,
        },
        QrLogoLayer {
            color: QR_LOGO_BLUE,
            path_data: "M163 500L75 500L75 0L163 0L163 500Z",
            translate_x: 3.263889,
            translate_y: 3.759921,
            scale: 0.00496032,
        },
        QrLogoLayer {
            color: QR_LOGO_RED,
            path_data: "M-36 555L-23 544C2 566 22 588 36 613C50 638 57 663 57 689C57 710 52 727 42 740C32 753 19 760 0 760C-17 760 -30 755 -40 745C-50 735 -55 722 -55 707C-55 691 -50 677 -40 666C-30 655 -17 650 -1 650L4 650C6 650 7 651 9 651C7 642 4 634 0 626C-4 618 -8 612 -12 606C-16 600 -21 594 -25 589C-29 584 -32 581 -35 578C-37 576 -38 574 -39 572C-40 570 -41 569 -41 567C-41 562 -39 558 -36 555Z",
            translate_x: 3.839286,
            translate_y: 3.769841,
            scale: 0.00496032,
        },
    ]
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn encode_qr_alphanumeric_h(source_text: &str) -> Result<QrCode, String> {
    Ok(minimum_by_penalty(&qr_mask_candidates_alphanumeric_h(
        source_text,
    )?))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|codes| codes.len() == 8))]
fn qr_mask_candidates_alphanumeric_h(source_text: &str) -> Result<Vec<QrCode>, String> {
    validate_alphanumeric_payload(source_text)?;
    let version = select_version(source_text)?;
    let data_codewords = make_data_codewords(version, source_text)?;
    let all_codewords = add_error_correction_and_interleave(version, &data_codewords);
    let mut data_bits = codewords_to_bits(&all_codewords);
    let base_build = draw_function_patterns(version, empty_build());
    let placements = data_placements(version, &base_build);
    data_bits.resize(placements.len(), false);
    let candidates = (0..=7)
        .map(|mask_value| {
            let dark_with_data = placements
                .iter()
                .copied()
                .zip(data_bits.iter().copied())
                .filter_map(|(coord, bit)| (bit != mask_bit(mask_value, coord)).then_some(coord))
                .fold(base_build.dark_modules.clone(), |mut acc, coord| {
                    acc.insert(coord);
                    acc
                });
            let with_format = draw_format_bits(version, mask_value, dark_with_data);
            let with_version = draw_version_bits(version, with_format);
            new!(QrCode {
                version,
                size: qr_size(version),
                mask: mask_value,
                dark_modules: with_version,
            })
        })
        .collect::<Vec<_>>();
    Ok(candidates)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn qr_code_svg_for_text(source_text: &str) -> Result<String, String> {
    encode_qr_alphanumeric_h(source_text).map(|qr_code| qr_code_svg(&qr_code))
}

#[requires(true)]
#[ensures(ret.contains("<svg"))]
pub fn qr_code_svg(qr_code: &QrCode) -> String {
    let quiet_zone = 4;
    let outer_size = qr_code.size + quiet_zone * 2;
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {outer_size} {outer_size}\" width=\"100%\" height=\"100%\" role=\"img\" aria-label=\"jo'au dialect QR code\">"
    );
    let _ = write!(
        svg,
        "<rect width=\"{outer_size}\" height=\"{outer_size}\" fill=\"#fff\"/>"
    );
    svg.push_str("<path fill=\"#000\" d=\"");
    svg.push_str(&qr_code_dark_module_path(quiet_zone, qr_code));
    svg.push_str("\"/>");
    svg.push_str(&render_logo_svg(qr_code));
    svg.push_str("</svg>");
    svg
}

#[requires(quiet_zone >= 0)]
#[ensures(true)]
fn qr_code_dark_module_path(quiet_zone: i32, qr_code: &QrCode) -> String {
    let mut path = String::new();
    for coord in &qr_code.dark_modules {
        let _ = write!(
            path,
            "M{},{}h1v1h-1z",
            coord.x + quiet_zone,
            coord.y + quiet_zone
        );
    }
    path
}

#[requires(qr_code.size > 0)]
#[ensures(true)]
fn render_logo_svg(qr_code: &QrCode) -> String {
    qr_logo_placement(qr_code)
        .map(|placement| render_logo_placement_svg(placement))
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(ret.contains("<rect"))]
fn render_logo_placement_svg(placement: QrLogoPlacement) -> String {
    let quiet_zone = 4.0;
    let left_x = quiet_zone + f64::from(placement.left);
    let top_y = quiet_zone + f64::from(placement.top);
    let badge_size = f64::from(placement.side);
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        decimal(left_x),
        decimal(top_y),
        decimal(badge_size),
        decimal(badge_size),
        QR_LOGO_BADGE_BACKGROUND
    );
    let _ = write!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#000\" stroke-width=\"{}\"/>",
        decimal(left_x + QR_LOGO_BORDER_WIDTH / 2.0),
        decimal(top_y + QR_LOGO_BORDER_WIDTH / 2.0),
        decimal(badge_size - QR_LOGO_BORDER_WIDTH),
        decimal(badge_size - QR_LOGO_BORDER_WIDTH),
        decimal(QR_LOGO_BORDER_WIDTH)
    );
    for layer in qr_logo_layers() {
        svg.push_str(&logo_path_layer(placement, layer));
    }
    svg
}

#[requires(true)]
#[ensures(ret.contains("<path"))]
fn logo_path_layer(placement: QrLogoPlacement, layer: QrLogoLayer) -> String {
    let quiet_zone = 4.0;
    let left_x = quiet_zone + f64::from(placement.left);
    let top_y = quiet_zone + f64::from(placement.top);
    let inner_x = left_x + QR_LOGO_INSET;
    let inner_y = top_y + QR_LOGO_INSET;
    let content_scale = qr_logo_content_scale(placement);
    format!(
        "<path fill=\"{}\" transform=\"translate({} {}) scale({} -{})\" d=\"{}\"/>",
        layer.color,
        decimal(inner_x + layer.translate_x * content_scale),
        decimal(inner_y + layer.translate_y * content_scale),
        decimal(layer.scale * content_scale),
        decimal(layer.scale * content_scale),
        layer.path_data
    )
}

#[requires(true)]
#[ensures(ret.is_none_or(|placement| placement.side >= QR_LOGO_MINIMUM_SIDE))]
fn qr_logo_placement(qr_code: &QrCode) -> Option<QrLogoPlacement> {
    qr_logo_placement_for_version(qr_code.version)
}

#[requires(true)]
#[ensures(ret > 0.0)]
fn qr_logo_content_scale(placement: QrLogoPlacement) -> f64 {
    (f64::from(placement.side) - QR_LOGO_INSET * 2.0) / QR_LOGO_BASE_CONTENT_SIZE
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret.is_none_or(|placement| placement.side >= QR_LOGO_MINIMUM_SIDE))]
fn qr_logo_placement_for_version(version: i32) -> Option<QrLogoPlacement> {
    let layout = qr_logo_layout_for_version(version);
    let mut candidate_sides = (QR_LOGO_MINIMUM_SIDE..=layout.size_value)
        .step_by(2)
        .filter(|side| {
            placement_fits_with_clearance(
                layout.size_value,
                centered_placement(layout.size_value, *side),
            )
        })
        .collect::<Vec<_>>();
    candidate_sides.reverse();

    candidate_sides
        .iter()
        .map(|side| centered_placement(layout.size_value, *side))
        .find(|placement| logo_placement_is_safe(&layout, *placement))
        .or_else(|| {
            candidate_sides
                .iter()
                .flat_map(|side| top_slot_placements(layout.size_value, *side))
                .find(|placement| logo_placement_is_safe(&layout, *placement))
        })
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret.version == version)]
fn qr_logo_layout_for_version(version: i32) -> QrLogoLayout {
    let build = draw_function_patterns(version, empty_build());
    let data_placement_coords = data_placements(version, &build);
    new!(QrLogoLayout {
        version,
        size_value: qr_size(version),
        function_modules: build.function_modules,
        data_placements: data_placement_coords,
        codeword_blocks: interleaved_codeword_blocks(version),
        correction_budget: ecc_codewords_per_block(version) / 2 - QR_LOGO_ERROR_CORRECTION_MARGIN,
    })
}

#[requires(true)]
#[ensures(true)]
fn logo_placement_is_safe(layout: &QrLogoLayout, placement: QrLogoPlacement) -> bool {
    placement_fits_with_clearance(layout.size_value, placement)
        && placement_has_function_clearance(&layout.function_modules, placement)
        && placement_fits_error_correction_budget(layout, placement)
}

#[requires(size_value > 0 && side >= QR_LOGO_MINIMUM_SIDE && side <= size_value)]
#[ensures(ret.side == side)]
fn centered_placement(size_value: i32, side: i32) -> QrLogoPlacement {
    new!(QrLogoPlacement {
        left: (size_value - side) / 2,
        top: (size_value - side) / 2,
        side,
    })
}

#[requires(size_value > 0 && side >= QR_LOGO_MINIMUM_SIDE && side <= size_value)]
#[ensures(ret.iter().all(|placement| placement.side == side))]
fn top_slot_placements(size_value: i32, side: i32) -> Vec<QrLogoPlacement> {
    let left = (size_value - side) / 2;
    let center = size_value / 2;
    let max_top = center - side;
    if max_top < QR_LOGO_FUNCTION_CLEARANCE {
        return Vec::new();
    }
    (QR_LOGO_FUNCTION_CLEARANCE..=max_top)
        .rev()
        .map(|top| new!(QrLogoPlacement { left, top, side }))
        .collect()
}

#[requires(size_value > 0)]
#[ensures(true)]
fn placement_fits_with_clearance(size_value: i32, placement: QrLogoPlacement) -> bool {
    placement.left >= QR_LOGO_FUNCTION_CLEARANCE
        && placement.top >= QR_LOGO_FUNCTION_CLEARANCE
        && placement.left + placement.side + QR_LOGO_FUNCTION_CLEARANCE <= size_value
        && placement.top + placement.side + QR_LOGO_FUNCTION_CLEARANCE <= size_value
}

#[requires(true)]
#[ensures(true)]
fn placement_has_function_clearance(
    function_modules: &BTreeSet<QrCoord>,
    placement: QrLogoPlacement,
) -> bool {
    let clearance = QR_LOGO_FUNCTION_CLEARANCE;
    (placement.top - clearance..placement.top + placement.side + clearance).all(|y| {
        (placement.left - clearance..placement.left + placement.side + clearance)
            .all(|x| !function_modules.contains(&QrCoord { x, y }))
    })
}

#[requires(true)]
#[ensures(true)]
fn placement_fits_error_correction_budget(
    layout: &QrLogoLayout,
    placement: QrLogoPlacement,
) -> bool {
    damaged_codewords_per_block(layout, placement)
        .into_iter()
        .max()
        .unwrap_or(0)
        <= layout.correction_budget
}

#[requires(true)]
#[ensures(ret.iter().all(|count| *count >= 0))]
fn damaged_codewords_per_block(layout: &QrLogoLayout, placement: QrLogoPlacement) -> Vec<i32> {
    let damaged_codewords = layout
        .data_placements
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(bit_index, coord)| {
            if coord_inside_placement(placement, coord) {
                let codeword_index = bit_index / 8;
                layout
                    .codeword_blocks
                    .get(codeword_index)
                    .copied()
                    .map(|block_index| (block_index, codeword_index))
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    (0..error_correction_block_count(layout.version))
        .map(|block_index| {
            damaged_codewords
                .iter()
                .filter(|(damaged_block, _)| *damaged_block == block_index as usize)
                .count() as i32
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn coord_inside_placement(placement: QrLogoPlacement, coord: QrCoord) -> bool {
    coord.x >= placement.left
        && coord.y >= placement.top
        && coord.x < placement.left + placement.side
        && coord.y < placement.top + placement.side
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn interleaved_codeword_blocks(version: i32) -> Vec<usize> {
    let ecc_length = ecc_codewords_per_block(version) as usize;
    let block_count = error_correction_block_count(version) as usize;
    let raw_codeword_count = raw_codewords_for_version(version) as usize;
    let short_block_length = raw_codeword_count / block_count;
    let short_block_count = block_count - raw_codeword_count % block_count;
    let short_data_length = short_block_length - ecc_length;
    let data_lengths = (0..block_count)
        .map(|block_index| short_data_length + usize::from(block_index >= short_block_count))
        .collect::<Vec<_>>();
    let max_data_length = data_lengths.iter().copied().max().unwrap_or(0);
    let data_codeword_blocks = (0..max_data_length)
        .flat_map(|index| {
            data_lengths
                .iter()
                .enumerate()
                .filter_map(move |(block_index, data_length)| {
                    (index < *data_length).then_some(block_index)
                })
        })
        .collect::<Vec<_>>();
    let error_codeword_blocks = (0..ecc_length)
        .flat_map(|_| 0..block_count)
        .collect::<Vec<_>>();
    data_codeword_blocks
        .into_iter()
        .chain(error_codeword_blocks)
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn select_version(source_text: &str) -> Result<i32, String> {
    (1..=40)
        .find(|version| can_fit(*version, source_text))
        .ok_or_else(|| {
            "QR payload is too large for a version 40-H alphanumeric QR code.".to_owned()
        })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn validate_alphanumeric_payload(source_text: &str) -> Result<(), String> {
    for ch in source_text.chars() {
        if alphanumeric_value(ch).is_none() {
            return Err(format!(
                "Character is not valid in QR alphanumeric mode: {ch}"
            ));
        }
    }
    Ok(())
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn can_fit(version: i32, source_text: &str) -> bool {
    alphanumeric_payload_bits(version, source_text)
        .map(|bits| bits.len() as i32 <= data_capacity_bits(version))
        .unwrap_or(false)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn make_data_codewords(version: i32, source_text: &str) -> Result<Vec<i32>, String> {
    let mut payload_bits = alphanumeric_payload_bits(version, source_text)?;
    let capacity = data_capacity_bits(version) as usize;
    if payload_bits.len() > capacity {
        return Err("QR payload does not fit in the selected QR version.".to_owned());
    }
    let terminator_length = 4usize.min(capacity - payload_bits.len());
    payload_bits.extend(std::iter::repeat_n(false, terminator_length));
    let padding_bits = (8 - payload_bits.len() % 8) % 8;
    payload_bits.extend(std::iter::repeat_n(false, padding_bits));
    let mut codewords = bits_to_codewords(&payload_bits);
    let data_codeword_count = data_codewords_for_version(version) as usize;
    let mut pad_index = 0usize;
    while codewords.len() < data_codeword_count {
        codewords.push(if pad_index.is_multiple_of(2) {
            0xEC
        } else {
            0x11
        });
        pad_index += 1;
    }
    codewords.truncate(data_codeword_count);
    Ok(codewords)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn alphanumeric_payload_bits(version: i32, source_text: &str) -> Result<Vec<bool>, String> {
    let values = source_text
        .chars()
        .map(|ch| {
            alphanumeric_value(ch)
                .ok_or_else(|| format!("Character is not valid in QR alphanumeric mode: {ch}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut bits = int_bits(4, 0x2);
    bits.extend(int_bits(character_count_bits(version), values.len() as i32));
    bits.extend(encode_alphanumeric_values(&values));
    Ok(bits)
}

#[requires(true)]
#[ensures(true)]
fn encode_alphanumeric_values(values: &[i32]) -> Vec<bool> {
    let mut bits = Vec::new();
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        bits.extend(int_bits(11, pair[0] * 45 + pair[1]));
    }
    if let [last] = chunks.remainder() {
        bits.extend(int_bits(6, *last));
    }
    bits
}

#[requires(true)]
#[ensures(true)]
fn alphanumeric_value(value: char) -> Option<i32> {
    ALPHANUMERIC_CHARACTERS
        .chars()
        .position(|candidate| candidate == value)
        .map(|index| index as i32)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn character_count_bits(version: i32) -> i32 {
    if version <= 9 {
        9
    } else if version <= 26 {
        11
    } else {
        13
    }
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn data_capacity_bits(version: i32) -> i32 {
    data_codewords_for_version(version) * 8
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn data_codewords_for_version(version: i32) -> i32 {
    raw_codewords_for_version(version)
        - ecc_codewords_per_block(version) * error_correction_block_count(version)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn raw_codewords_for_version(version: i32) -> i32 {
    num_raw_data_modules(version) / 8
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn num_raw_data_modules(version: i32) -> i32 {
    // ISO/IEC 18004:2015 gives this closed form after subtracting finder,
    // timing, alignment, format, version, and fixed dark modules.
    let base = (16 * version + 128) * version + 64;
    if version < 2 {
        base
    } else {
        let alignment_count = version / 7 + 2;
        let without_alignment = base - ((25 * alignment_count - 10) * alignment_count - 55);
        if version >= 7 {
            without_alignment - 36
        } else {
            without_alignment
        }
    }
}

#[requires(version >= 1 && version <= 40)]
#[requires(codewords.len() == data_codewords_for_version(version) as usize)]
#[ensures(true)]
fn add_error_correction_and_interleave(version: i32, codewords: &[i32]) -> Vec<i32> {
    let ecc_length = ecc_codewords_per_block(version) as usize;
    let block_count = error_correction_block_count(version) as usize;
    let raw_codeword_count = raw_codewords_for_version(version) as usize;
    let short_block_length = raw_codeword_count / block_count;
    let short_block_count = block_count - raw_codeword_count % block_count;
    let short_data_length = short_block_length - ecc_length;
    let divisor = reed_solomon_divisor(ecc_length);
    let mut blocks = Vec::new();
    let mut offset = 0usize;
    for block_index in 0..block_count {
        let data_length = short_data_length + usize::from(block_index >= short_block_count);
        let block_data = codewords[offset..offset + data_length].to_vec();
        offset += data_length;
        let error_codewords = reed_solomon_remainder(&divisor, &block_data);
        blocks.push(new!(QrBlock {
            data_codewords: block_data,
            error_codewords,
        }));
    }
    let max_data_length = blocks
        .iter()
        .map(|block| block.data_codewords.len())
        .max()
        .unwrap_or(0);
    let data = (0..max_data_length)
        .flat_map(|index| {
            blocks
                .iter()
                .filter_map(move |block| block.data_codewords.get(index).copied())
        })
        .collect::<Vec<_>>();
    let error = (0..ecc_length)
        .flat_map(|index| {
            blocks
                .iter()
                .filter_map(move |block| block.error_codewords.get(index).copied())
        })
        .collect::<Vec<_>>();
    data.into_iter().chain(error).collect()
}

#[requires(degree > 0)]
#[ensures(ret.len() == degree)]
fn reed_solomon_divisor(degree: usize) -> Vec<i32> {
    let mut coefficients = vec![0; degree - 1];
    coefficients.push(1);
    let mut root = 1;
    for _ in 0..degree {
        coefficients = coefficients
            .iter()
            .enumerate()
            .map(|(index, coefficient)| {
                reed_solomon_multiply(*coefficient, root)
                    ^ coefficients.get(index + 1).copied().unwrap_or(0)
            })
            .collect();
        root = reed_solomon_multiply(root, 0x02);
    }
    coefficients
}

#[requires(true)]
#[ensures(ret.len() == divisor.len())]
fn reed_solomon_remainder(divisor: &[i32], codewords: &[i32]) -> Vec<i32> {
    codewords
        .iter()
        .fold(vec![0; divisor.len()], |result, codeword| {
            if let Some((&first, rest)) = result.split_first() {
                let factor = codeword ^ first;
                let shifted = rest.iter().copied().chain(std::iter::once(0));
                shifted
                    .zip(divisor.iter().copied())
                    .map(|(shifted_value, divisor_value)| {
                        shifted_value ^ reed_solomon_multiply(divisor_value, factor)
                    })
                    .collect()
            } else {
                Vec::new()
            }
        })
}

#[requires(left >= 0 && right >= 0)]
#[ensures(ret >= 0 && ret <= 0xff)]
fn reed_solomon_multiply(left: i32, right: i32) -> i32 {
    (0..=7).fold(0, |acc, bit_index| {
        if test_bit(right, bit_index) {
            acc ^ gf_multiply_x_power(left, bit_index)
        } else {
            acc
        }
    })
}

#[requires(value >= 0 && power >= 0)]
#[ensures(ret >= 0 && ret <= 0xff)]
fn gf_multiply_x_power(value: i32, power: i32) -> i32 {
    (0..power).fold(value, |current, _| {
        let shifted = current << 1;
        if shifted & 0x100 != 0 {
            // ISO/IEC 18004:2015 Annex B uses GF(256) with primitive polynomial 0x11D.
            (shifted ^ 0x11D) & 0xFF
        } else {
            shifted & 0xFF
        }
    })
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_function_patterns(version: i32, build: QrBuild) -> QrBuild {
    let size_value = qr_size(version);
    let build = draw_finder_pattern(version, QrCoord { x: 3, y: 3 }, build);
    let build = draw_finder_pattern(
        version,
        QrCoord {
            x: size_value - 4,
            y: 3,
        },
        build,
    );
    let build = draw_finder_pattern(
        version,
        QrCoord {
            x: 3,
            y: size_value - 4,
        },
        build,
    );
    let build = draw_timing_patterns(version, build);
    let build = draw_alignment_patterns(version, build);
    let build = draw_initial_format_areas(version, build);
    draw_dark_module(version, build)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_finder_pattern(version: i32, center: QrCoord, build: QrBuild) -> QrBuild {
    (center.y - 4..=center.y + 4)
        .flat_map(|y| (center.x - 4..=center.x + 4).map(move |x| QrCoord { x, y }))
        .fold(build, |acc, coord| {
            let distance = (coord.x - center.x).abs().max((coord.y - center.y).abs());
            let dark = distance != 2 && distance != 4;
            set_function_module(version, coord, dark, acc)
        })
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_timing_patterns(version: i32, build: QrBuild) -> QrBuild {
    (8..=qr_size(version) - 9).fold(build, |acc, index| {
        let dark = index % 2 == 0;
        let acc = set_function_module(version, QrCoord { x: 6, y: index }, dark, acc);
        set_function_module(version, QrCoord { x: index, y: 6 }, dark, acc)
    })
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_alignment_patterns(version: i32, build: QrBuild) -> QrBuild {
    let positions = alignment_pattern_positions(version);
    let mut centers = Vec::new();
    for y in &positions {
        for x in &positions {
            if !alignment_pattern_overlaps_finder(&positions, *x, *y) {
                centers.push(QrCoord { x: *x, y: *y });
            }
        }
    }
    centers.into_iter().fold(build, |acc, center| {
        (center.y - 2..=center.y + 2)
            .flat_map(|y| (center.x - 2..=center.x + 2).map(move |x| QrCoord { x, y }))
            .fold(acc, |next, coord| {
                let distance = (coord.x - center.x).abs().max((coord.y - center.y).abs());
                set_function_module(version, coord, distance != 1, next)
            })
    })
}

#[requires(true)]
#[ensures(true)]
fn alignment_pattern_overlaps_finder(positions: &[i32], x: i32, y: i32) -> bool {
    let Some(first_position) = positions.first().copied() else {
        return false;
    };
    let Some(last_position) = positions.last().copied() else {
        return false;
    };
    (x == first_position && y == first_position)
        || (x == last_position && y == first_position)
        || (x == first_position && y == last_position)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_dark_module(version: i32, build: QrBuild) -> QrBuild {
    set_function_module(
        version,
        QrCoord {
            x: 8,
            y: 4 * version + 9,
        },
        true,
        build,
    )
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_initial_format_areas(version: i32, build: QrBuild) -> QrBuild {
    format_coords(version)
        .into_iter()
        .chain(version_coords(version))
        .fold(build, |acc, coord| {
            set_function_module(version, coord, false, acc)
        })
}

#[requires(version >= 1 && version <= 40 && mask_value >= 0 && mask_value <= 7)]
#[ensures(true)]
fn draw_format_bits(
    version: i32,
    mask_value: i32,
    mut dark_modules: BTreeSet<QrCoord>,
) -> BTreeSet<QrCoord> {
    let size_value = qr_size(version);
    let data_value = 0x10 | mask_value;
    let bits = format_bits(data_value);
    let placements = (0..=5)
        .map(|i| (QrCoord { x: 8, y: i }, i))
        .chain((6..=7).map(|i| (QrCoord { x: 8, y: i + 1 }, i)))
        .chain((8..=14).map(|i| {
            (
                QrCoord {
                    x: 8,
                    y: size_value - 15 + i,
                },
                i,
            )
        }))
        .chain((0..=7).map(|i| {
            (
                QrCoord {
                    x: size_value - 1 - i,
                    y: 8,
                },
                i,
            )
        }))
        .chain(std::iter::once((QrCoord { x: 7, y: 8 }, 8)))
        .chain((9..=14).map(|i| (QrCoord { x: 14 - i, y: 8 }, i)));
    for (coord, bit_index) in placements {
        if test_bit(bits, bit_index) {
            dark_modules.insert(coord);
        } else {
            dark_modules.remove(&coord);
        }
    }
    dark_modules
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn draw_version_bits(version: i32, mut dark_modules: BTreeSet<QrCoord>) -> BTreeSet<QrCoord> {
    if version < 7 {
        return dark_modules;
    }
    let size_value = qr_size(version);
    let bits = version_bits(version);
    let placements = (0..=17)
        .map(|i| {
            (
                QrCoord {
                    x: size_value - 11 + i % 3,
                    y: i / 3,
                },
                i,
            )
        })
        .chain((0..=17).map(|i| {
            (
                QrCoord {
                    x: i / 3,
                    y: size_value - 11 + i % 3,
                },
                i,
            )
        }));
    for (coord, bit_index) in placements {
        if test_bit(bits, bit_index) {
            dark_modules.insert(coord);
        } else {
            dark_modules.remove(&coord);
        }
    }
    dark_modules
}

#[requires(data_value >= 0)]
#[ensures(ret >= 0)]
fn format_bits(data_value: i32) -> i32 {
    // ISO/IEC 18004:2015 Annex C uses BCH generator 0x537 and mask 0x5412.
    let remainder = (1..=10).fold(data_value, |acc, _| {
        let shifted = acc << 1;
        if shifted >> 10 != 0 {
            shifted ^ 0x537
        } else {
            shifted
        }
    });
    ((data_value << 10) | remainder) ^ 0x5412
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret >= 0)]
fn version_bits(version: i32) -> i32 {
    // ISO/IEC 18004:2015 Annex D uses BCH version generator 0x1F25.
    let remainder = (1..=12).fold(version, |acc, _| {
        let shifted = acc << 1;
        if shifted >> 12 != 0 {
            shifted ^ 0x1F25
        } else {
            shifted
        }
    });
    (version << 12) | remainder
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn format_coords(version: i32) -> Vec<QrCoord> {
    let size_value = qr_size(version);
    (0..=8)
        .filter(|y| *y != 6)
        .map(|y| QrCoord { x: 8, y })
        .chain((0..=8).filter(|x| *x != 6).map(|x| QrCoord { x, y: 8 }))
        .chain((size_value - 8..=size_value - 1).map(|y| QrCoord { x: 8, y }))
        .chain((size_value - 8..=size_value - 1).map(|x| QrCoord { x, y: 8 }))
        .collect()
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn version_coords(version: i32) -> Vec<QrCoord> {
    if version < 7 {
        return Vec::new();
    }
    let size_value = qr_size(version);
    (0..=5)
        .flat_map(|y| (size_value - 11..=size_value - 9).map(move |x| QrCoord { x, y }))
        .chain(
            (size_value - 11..=size_value - 9).flat_map(|y| (0..=5).map(move |x| QrCoord { x, y })),
        )
        .collect()
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn data_placements(version: i32, build: &QrBuild) -> Vec<QrCoord> {
    let size_value = qr_size(version);
    let mut column_pair_rights = Vec::new();
    let mut right = size_value - 1;
    while right > 0 {
        if right == 6 {
            right = 5;
        }
        column_pair_rights.push(right);
        right -= 2;
    }
    column_pair_rights
        .into_iter()
        .flat_map(|right| {
            let rows = if ((right + 1) & 2) == 0 {
                (0..=size_value - 1).rev().collect::<Vec<_>>()
            } else {
                (0..=size_value - 1).collect::<Vec<_>>()
            };
            rows.into_iter().flat_map(move |y| {
                [right, right - 1]
                    .into_iter()
                    .map(move |x| QrCoord { x, y })
            })
        })
        .filter(|coord| !build.function_modules.contains(coord))
        .collect()
}

#[requires(mask_value >= 0 && mask_value <= 7)]
#[ensures(true)]
fn mask_bit(mask_value: i32, coord: QrCoord) -> bool {
    match mask_value {
        0 => (coord.x + coord.y) % 2 == 0,
        1 => coord.y % 2 == 0,
        2 => coord.x % 3 == 0,
        3 => (coord.x + coord.y) % 3 == 0,
        4 => (coord.x / 3 + coord.y / 2) % 2 == 0,
        5 => (coord.x * coord.y) % 2 + (coord.x * coord.y) % 3 == 0,
        6 => ((coord.x * coord.y) % 2 + (coord.x * coord.y) % 3) % 2 == 0,
        7 => ((coord.x + coord.y) % 2 + (coord.x * coord.y) % 3) % 2 == 0,
        _ => false,
    }
}

#[requires(!codes.is_empty())]
#[ensures(true)]
fn minimum_by_penalty(codes: &[QrCode]) -> QrCode {
    let mut candidates = codes.iter();
    let first = candidates
        .next()
        .expect("requires non-empty QR code candidates");
    let mut best = first;
    let mut best_penalty = qr_penalty(first);
    for candidate in candidates {
        let penalty = qr_penalty(candidate);
        if penalty < best_penalty {
            best = candidate;
            best_penalty = penalty;
        }
    }
    best.clone()
}

#[requires(qr_code.size > 0)]
#[ensures(ret >= 0)]
fn qr_penalty(qr_code: &QrCode) -> i32 {
    let rows = (0..qr_code.size)
        .map(|y| {
            (0..qr_code.size)
                .map(|x| module_dark(qr_code, QrCoord { x, y }))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let columns = (0..qr_code.size)
        .map(|x| {
            (0..qr_code.size)
                .map(|y| module_dark(qr_code, QrCoord { x, y }))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    consecutive_penalty(&rows)
        + consecutive_penalty(&columns)
        + block_penalty(qr_code)
        + finder_penalty(&rows)
        + finder_penalty(&columns)
        + balance_penalty(qr_code)
}

#[requires(true)]
#[ensures(ret >= 0)]
fn consecutive_penalty(lines: &[Vec<bool>]) -> i32 {
    lines
        .iter()
        .flat_map(|line| run_lengths(line))
        .filter(|run_length| *run_length >= 5)
        .map(|run_length| 3 + run_length - 5)
        .sum()
}

#[requires(true)]
#[ensures(ret.iter().all(|length| *length > 0))]
fn run_lengths(values: &[bool]) -> Vec<i32> {
    let Some((&first, rest)) = values.split_first() else {
        return Vec::new();
    };
    let mut previous = first;
    let mut lengths = vec![1];
    for current in rest {
        if *current == previous {
            if let Some(last) = lengths.last_mut() {
                *last += 1;
            }
        } else {
            previous = *current;
            lengths.push(1);
        }
    }
    lengths
}

#[requires(qr_code.size > 0)]
#[ensures(ret >= 0)]
fn block_penalty(qr_code: &QrCode) -> i32 {
    let mut penalty = 0;
    for y in 0..qr_code.size - 1 {
        for x in 0..qr_code.size - 1 {
            let color = module_dark(qr_code, QrCoord { x, y });
            if module_dark(qr_code, QrCoord { x: x + 1, y }) == color
                && module_dark(qr_code, QrCoord { x, y: y + 1 }) == color
                && module_dark(qr_code, QrCoord { x: x + 1, y: y + 1 }) == color
            {
                penalty += 3;
            }
        }
    }
    penalty
}

#[requires(true)]
#[ensures(ret >= 0)]
fn finder_penalty(lines: &[Vec<bool>]) -> i32 {
    const PATTERN_A: [bool; 11] = [
        true, false, true, true, true, false, true, false, false, false, false,
    ];
    const PATTERN_B: [bool; 11] = [
        false, false, false, false, true, false, true, true, true, false, true,
    ];
    lines
        .iter()
        .map(|line| finder_line_penalty(line, &PATTERN_A, &PATTERN_B))
        .sum()
}

#[requires(pattern_a.len() == 11)]
#[requires(pattern_b.len() == 11)]
#[ensures(ret >= 0)]
fn finder_line_penalty(line: &[bool], pattern_a: &[bool], pattern_b: &[bool]) -> i32 {
    let size = line.len() as i32;
    (-4..=size - 7)
        .filter(|window_start| {
            finder_window_matches(line, *window_start, pattern_a)
                || finder_window_matches(line, *window_start, pattern_b)
        })
        .count() as i32
        * 40
}

#[requires(pattern.len() == 11)]
#[ensures(true)]
fn finder_window_matches(line: &[bool], window_start: i32, pattern: &[bool]) -> bool {
    // ISO/IEC 18004:2015(E) 7.8.3.1 Table 11 N3 counts finder-like
    // 1:1:3:1:1 runs preceded or followed by four light modules. A run at
    // a symbol edge can use the outside-of-symbol area as that light run.
    pattern.iter().enumerate().all(|(offset, expected)| {
        module_dark_or_light_padding(line, window_start + offset as i32) == *expected
    })
}

#[requires(true)]
#[ensures(true)]
fn module_dark_or_light_padding(line: &[bool], index: i32) -> bool {
    if index < 0 {
        return false;
    }
    line.get(index as usize).copied().unwrap_or(false)
}

#[requires(qr_code.size > 0)]
#[ensures(ret >= 0)]
fn balance_penalty(qr_code: &QrCode) -> i32 {
    let dark_count = qr_code.dark_modules.len() as f64;
    let total_count = f64::from(qr_code.size * qr_code.size);
    let percent = dark_count * 100.0 / total_count;
    let lower = (percent / 5.0).floor() * 5.0;
    let upper = lower + 5.0;
    ((lower - 50.0).abs().min((upper - 50.0).abs()) / 5.0).round() as i32 * 10
}

#[requires(true)]
#[ensures(true)]
fn module_dark(qr_code: &QrCode, coord: QrCoord) -> bool {
    qr_code.dark_modules.contains(&coord)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn set_function_module(version: i32, coord: QrCoord, dark: bool, mut build: QrBuild) -> QrBuild {
    if !coord_in_bounds(version, coord) {
        return build;
    }
    if dark {
        build.dark_modules.insert(coord);
    } else {
        build.dark_modules.remove(&coord);
    }
    build.function_modules.insert(coord);
    build
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn coord_in_bounds(version: i32, coord: QrCoord) -> bool {
    coord.x >= 0 && coord.y >= 0 && coord.x < qr_size(version) && coord.y < qr_size(version)
}

#[requires(version >= 1 && version <= 40)]
#[ensures(true)]
fn alignment_pattern_positions(version: i32) -> Vec<i32> {
    if version == 1 {
        return Vec::new();
    }
    let size_value = qr_size(version);
    let count = version / 7 + 2;
    let step = if version == 32 {
        26
    } else {
        ((version * 4 + count * 2 + 1) / (count * 2 - 2)) * 2
    };
    let mut middle_positions = Vec::new();
    let mut value = size_value - 7;
    while middle_positions.len() < (count - 1) as usize {
        middle_positions.push(value);
        value -= step;
    }
    middle_positions.reverse();
    std::iter::once(6).chain(middle_positions).collect()
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn qr_size(version: i32) -> i32 {
    version * 4 + 17
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn ecc_codewords_per_block(version: i32) -> i32 {
    HIGH_ECC_CODEWORDS_PER_BLOCK[version as usize]
}

#[requires(version >= 1 && version <= 40)]
#[ensures(ret > 0)]
fn error_correction_block_count(version: i32) -> i32 {
    HIGH_ERROR_CORRECTION_BLOCK_COUNT[version as usize]
}

#[requires(true)]
#[ensures(ret.dark_modules.is_empty() && ret.function_modules.is_empty())]
fn empty_build() -> QrBuild {
    QrBuild {
        dark_modules: BTreeSet::new(),
        function_modules: BTreeSet::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn codewords_to_bits(codewords: &[i32]) -> Vec<bool> {
    codewords
        .iter()
        .flat_map(|codeword| int_bits(8, *codeword))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn bits_to_codewords(bits: &[bool]) -> Vec<i32> {
    bits.chunks(8).map(bits_to_int).collect()
}

#[requires(width >= 0)]
#[requires(width < 63)]
#[requires(value >= 0 && i64::from(value) < (1_i64 << width))]
#[ensures(ret.len() == width as usize)]
fn int_bits(width: i32, value: i32) -> Vec<bool> {
    (0..width)
        .rev()
        .map(|bit_index| test_bit(value, bit_index))
        .collect()
}

#[requires(true)]
#[ensures(ret >= 0)]
fn bits_to_int(bits: &[bool]) -> i32 {
    bits.iter().fold(0, |acc, bit| acc * 2 + i32::from(*bit))
}

#[requires(bit_index >= 0)]
#[ensures(true)]
fn test_bit(value: i32, bit_index: i32) -> bool {
    value & (1 << bit_index) != 0
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn decimal(value: f64) -> String {
    let normalized = if value.abs() < 0.0000005 { 0.0 } else { value };
    let rendered = format!("{normalized:.6}");
    let trimmed = rendered.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[allow(unused_imports)]
    use bityzba::ensures;
    use bityzba::requires;

    const SMALL_JOHAU_QR_PAYLOAD: &str = "WEB+JOHAU:-C.LAHU*LAHE+DIHU";

    // Independent reference generated with Segno 1.6.6:
    // UV_CACHE_DIR=/tmp/jbotci-uv-cache uv run --with segno==1.6.6 python - <<'PY'
    // import segno
    // payload = 'WEB+JOHAU:-C.LAHU*LAHE+DIHU'
    // qr = segno.make(payload, error='h', mode='alphanumeric', mask=5, micro=False, boost_error=False)
    // Segno defaults to mask 7 for this payload; mask 5 is forced here to
    // compare the matrix for the mask selected by this encoder.
    // for y, row in enumerate(qr.matrix):
    //     for x, value in enumerate(row):
    //         if value:
    //             print(f'({x}, {y})')
    // PY
    const SMALL_JOHAU_QR_SEGNO_DARK_MODULES: &[(i32, i32)] = &[
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 0),
        (8, 0),
        (10, 0),
        (11, 0),
        (12, 0),
        (19, 0),
        (20, 0),
        (22, 0),
        (23, 0),
        (24, 0),
        (25, 0),
        (26, 0),
        (27, 0),
        (28, 0),
        (0, 1),
        (6, 1),
        (12, 1),
        (13, 1),
        (14, 1),
        (18, 1),
        (20, 1),
        (22, 1),
        (28, 1),
        (0, 2),
        (2, 2),
        (3, 2),
        (4, 2),
        (6, 2),
        (8, 2),
        (9, 2),
        (10, 2),
        (11, 2),
        (12, 2),
        (13, 2),
        (14, 2),
        (16, 2),
        (17, 2),
        (22, 2),
        (24, 2),
        (25, 2),
        (26, 2),
        (28, 2),
        (0, 3),
        (2, 3),
        (3, 3),
        (4, 3),
        (6, 3),
        (9, 3),
        (11, 3),
        (12, 3),
        (14, 3),
        (15, 3),
        (20, 3),
        (22, 3),
        (24, 3),
        (25, 3),
        (26, 3),
        (28, 3),
        (0, 4),
        (2, 4),
        (3, 4),
        (4, 4),
        (6, 4),
        (8, 4),
        (10, 4),
        (12, 4),
        (16, 4),
        (17, 4),
        (19, 4),
        (20, 4),
        (22, 4),
        (24, 4),
        (25, 4),
        (26, 4),
        (28, 4),
        (0, 5),
        (6, 5),
        (9, 5),
        (10, 5),
        (13, 5),
        (15, 5),
        (17, 5),
        (19, 5),
        (22, 5),
        (28, 5),
        (0, 6),
        (1, 6),
        (2, 6),
        (3, 6),
        (4, 6),
        (5, 6),
        (6, 6),
        (8, 6),
        (10, 6),
        (12, 6),
        (14, 6),
        (16, 6),
        (18, 6),
        (20, 6),
        (22, 6),
        (23, 6),
        (24, 6),
        (25, 6),
        (26, 6),
        (27, 6),
        (28, 6),
        (8, 7),
        (9, 7),
        (11, 7),
        (12, 7),
        (16, 7),
        (17, 7),
        (18, 7),
        (20, 7),
        (5, 8),
        (6, 8),
        (12, 8),
        (14, 8),
        (15, 8),
        (16, 8),
        (19, 8),
        (22, 8),
        (24, 8),
        (26, 8),
        (28, 8),
        (0, 9),
        (1, 9),
        (4, 9),
        (5, 9),
        (7, 9),
        (12, 9),
        (13, 9),
        (14, 9),
        (18, 9),
        (21, 9),
        (23, 9),
        (24, 9),
        (25, 9),
        (26, 9),
        (0, 10),
        (1, 10),
        (3, 10),
        (5, 10),
        (6, 10),
        (8, 10),
        (12, 10),
        (13, 10),
        (15, 10),
        (16, 10),
        (17, 10),
        (18, 10),
        (19, 10),
        (22, 10),
        (23, 10),
        (26, 10),
        (28, 10),
        (3, 11),
        (4, 11),
        (7, 11),
        (9, 11),
        (10, 11),
        (11, 11),
        (12, 11),
        (13, 11),
        (15, 11),
        (16, 11),
        (17, 11),
        (19, 11),
        (20, 11),
        (21, 11),
        (25, 11),
        (26, 11),
        (28, 11),
        (1, 12),
        (6, 12),
        (7, 12),
        (9, 12),
        (13, 12),
        (22, 12),
        (25, 12),
        (26, 12),
        (27, 12),
        (0, 13),
        (3, 13),
        (5, 13),
        (8, 13),
        (15, 13),
        (16, 13),
        (17, 13),
        (18, 13),
        (20, 13),
        (21, 13),
        (22, 13),
        (24, 13),
        (25, 13),
        (27, 13),
        (28, 13),
        (0, 14),
        (1, 14),
        (2, 14),
        (3, 14),
        (5, 14),
        (6, 14),
        (8, 14),
        (9, 14),
        (10, 14),
        (12, 14),
        (15, 14),
        (17, 14),
        (18, 14),
        (19, 14),
        (20, 14),
        (21, 14),
        (22, 14),
        (24, 14),
        (1, 15),
        (2, 15),
        (9, 15),
        (10, 15),
        (12, 15),
        (14, 15),
        (17, 15),
        (18, 15),
        (22, 15),
        (26, 15),
        (27, 15),
        (0, 16),
        (2, 16),
        (6, 16),
        (9, 16),
        (10, 16),
        (13, 16),
        (19, 16),
        (20, 16),
        (21, 16),
        (23, 16),
        (26, 16),
        (0, 17),
        (7, 17),
        (8, 17),
        (9, 17),
        (11, 17),
        (12, 17),
        (15, 17),
        (24, 17),
        (27, 17),
        (0, 18),
        (1, 18),
        (3, 18),
        (4, 18),
        (5, 18),
        (6, 18),
        (7, 18),
        (8, 18),
        (10, 18),
        (12, 18),
        (13, 18),
        (15, 18),
        (17, 18),
        (18, 18),
        (19, 18),
        (21, 18),
        (22, 18),
        (25, 18),
        (0, 19),
        (2, 19),
        (3, 19),
        (4, 19),
        (5, 19),
        (8, 19),
        (12, 19),
        (13, 19),
        (15, 19),
        (17, 19),
        (22, 19),
        (25, 19),
        (26, 19),
        (27, 19),
        (0, 20),
        (4, 20),
        (6, 20),
        (7, 20),
        (14, 20),
        (15, 20),
        (17, 20),
        (18, 20),
        (20, 20),
        (21, 20),
        (22, 20),
        (23, 20),
        (24, 20),
        (25, 20),
        (26, 20),
        (28, 20),
        (8, 21),
        (9, 21),
        (10, 21),
        (14, 21),
        (15, 21),
        (19, 21),
        (20, 21),
        (24, 21),
        (25, 21),
        (27, 21),
        (28, 21),
        (0, 22),
        (1, 22),
        (2, 22),
        (3, 22),
        (4, 22),
        (5, 22),
        (6, 22),
        (11, 22),
        (12, 22),
        (13, 22),
        (15, 22),
        (19, 22),
        (20, 22),
        (22, 22),
        (24, 22),
        (26, 22),
        (28, 22),
        (0, 23),
        (6, 23),
        (8, 23),
        (9, 23),
        (10, 23),
        (18, 23),
        (20, 23),
        (24, 23),
        (25, 23),
        (26, 23),
        (28, 23),
        (0, 24),
        (2, 24),
        (3, 24),
        (4, 24),
        (6, 24),
        (12, 24),
        (13, 24),
        (15, 24),
        (18, 24),
        (19, 24),
        (20, 24),
        (21, 24),
        (22, 24),
        (23, 24),
        (24, 24),
        (26, 24),
        (28, 24),
        (0, 25),
        (2, 25),
        (3, 25),
        (4, 25),
        (6, 25),
        (9, 25),
        (14, 25),
        (15, 25),
        (21, 25),
        (22, 25),
        (23, 25),
        (24, 25),
        (26, 25),
        (0, 26),
        (2, 26),
        (3, 26),
        (4, 26),
        (6, 26),
        (9, 26),
        (12, 26),
        (17, 26),
        (18, 26),
        (20, 26),
        (21, 26),
        (22, 26),
        (23, 26),
        (27, 26),
        (0, 27),
        (6, 27),
        (10, 27),
        (15, 27),
        (18, 27),
        (20, 27),
        (21, 27),
        (23, 27),
        (0, 28),
        (1, 28),
        (2, 28),
        (3, 28),
        (4, 28),
        (5, 28),
        (6, 28),
        (10, 28),
        (15, 28),
        (16, 28),
        (19, 28),
        (20, 28),
        (21, 28),
        (22, 28),
        (26, 28),
        (27, 28),
    ];

    #[requires(true)]
    #[ensures(true)]
    fn dark_modules_from_coords(coords: &[(i32, i32)]) -> BTreeSet<QrCoord> {
        coords
            .iter()
            .map(|(x, y)| QrCoord { x: *x, y: *y })
            .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn small_johau_qr_matches_independent_segno_matrix() {
        let qr = encode_qr_alphanumeric_h(SMALL_JOHAU_QR_PAYLOAD).expect("QR code");

        assert_eq!(qr.version, 3);
        assert_eq!(qr.size, 29);
        assert_eq!(qr.mask, 5);
        assert_eq!(
            qr.dark_modules,
            dark_modules_from_coords(SMALL_JOHAU_QR_SEGNO_DARK_MODULES)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selected_qr_mask_has_minimum_penalty() {
        let candidates =
            qr_mask_candidates_alphanumeric_h(SMALL_JOHAU_QR_PAYLOAD).expect("QR candidates");
        let qr = encode_qr_alphanumeric_h(SMALL_JOHAU_QR_PAYLOAD).expect("QR code");
        let selected_penalty = qr_penalty(&qr);
        let minimum_penalty = candidates
            .iter()
            .map(qr_penalty)
            .min()
            .expect("candidate set is non-empty");

        assert_eq!(candidates.len(), 8);
        assert_eq!(selected_penalty, minimum_penalty);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn format_bits_match_published_vectors() {
        assert_eq!(format_bits(0b00000), 0b101010000010010);
        assert_eq!(format_bits(0b01100), 0b110011000101111);
        assert_eq!(format_bits(0b10000), 0b001011010001001);
        assert_eq!(format_bits(0b10111), 0b000100000111011);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn version_bits_match_published_version_7_vector() {
        assert_eq!(version_bits(7), 0x07C94);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reed_solomon_remainder_matches_published_vector() {
        let divisor = reed_solomon_divisor(10);
        let remainder = reed_solomon_remainder(&divisor, &[32, 91, 11, 98, 56]);

        assert_eq!(remainder, vec![107, 33, 43, 244, 102, 30, 52, 87, 107, 207]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn encodes_small_johau_qr_with_expected_logo_placement() {
        let qr = encode_qr_alphanumeric_h(SMALL_JOHAU_QR_PAYLOAD).expect("QR code");
        assert_eq!(qr.version, 3);
        assert_eq!(qr.size, 29);
        assert_eq!(
            qr_logo_placement(&qr),
            Some(new!(QrLogoPlacement {
                left: 10,
                top: 10,
                side: 9
            }))
        );
        let svg = qr_code_svg(&qr);
        assert!(svg.contains("viewBox=\"0 0 37 37\""));
        assert!(
            svg.contains("<rect x=\"14\" y=\"14\" width=\"9\" height=\"9\" fill=\"#cbd4ff\"/>")
        );
        assert!(svg.contains("<rect x=\"14.125\" y=\"14.125\" width=\"8.75\" height=\"8.75\" fill=\"none\" stroke=\"#000\" stroke-width=\"0.25\"/>"));
        assert!(svg.contains("scale(0.006944 -0.006944)"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn encodes_larger_johau_qr_with_expected_logo_placement() {
        let payload = "WEB+JOHAU:-CKT.JOHU-JEI.-DS.BUHU-ZAI.KOHOI-KOI.SIHAU-SIHU.ZUHAI-SEHE.MOIHOI-GEI.LAUHU-LAU.-V.XUHU-PO.KUHAU-POHE";
        let qr = encode_qr_alphanumeric_h(payload).expect("QR code");
        assert_eq!(qr.version, 8);
        assert_eq!(
            qr_logo_placement(&qr),
            Some(new!(QrLogoPlacement {
                left: 19,
                top: 10,
                side: 11
            }))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_qr_alphanumeric_character_errors_before_version_search() {
        let error = encode_qr_alphanumeric_h("WEB+JOHAU:-na").expect_err("lowercase is invalid");

        assert_eq!(error, "Character is not valid in QR alphanumeric mode: n");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn n3_penalty_treats_outside_symbol_as_light_before_edge_pattern() {
        let line = vec![true, false, true, true, true, false, true, true];

        assert_eq!(finder_penalty(&[line]), 40);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn n3_penalty_treats_outside_symbol_as_light_after_edge_pattern() {
        let line = vec![
            true, true, true, true, true, false, true, true, true, false, true,
        ];

        assert_eq!(finder_penalty(&[line]), 40);
    }
}

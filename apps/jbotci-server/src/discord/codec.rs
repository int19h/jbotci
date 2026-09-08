//! Restart-safe request-state codec.
//!
//! A published Discord message carries its request in two places, both
//! returned verbatim inside every later interaction payload:
//!
//! * the gear button's `custom_id` holds the [`RequestHeader`]: schema
//!   version, tool, where the source lives, which optional fields are present,
//!   the packed options, page, revision, initiator, build tag and an integrity
//!   digest of the source fields;
//! * the deliberate input Text Display (or, for long input, the readable
//!   `jbotci-input.txt` attachment) holds the [input block](encode_input_block):
//!   the exact source fields in a reversible, Markdown-escaped, line-oriented
//!   form terminated by the status line.
//!
//! Nothing in memory or on disk is needed to rebuild a request. The digest is
//! integrity data only: it detects a transport mutation of the source and turns
//! it into an explicit error instead of a silently different request.

use std::fmt;

use base64::Engine;
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires, try_new};
use jbotci_cll::escape_discord_markdown_line;
use sha2::{Digest, Sha256};

use super::request::{
    BuildTag, DiscordRequest, DiscordTool, PageNumber, PublishedRequest, RequestStateError,
    Revision, Snowflake, SourceField, SourceStore, SourceText, utf16_len,
};

/// Custom ID schema version. Anything else is refused with an upgrade error.
pub(crate) const SCHEMA_VERSION: &str = "j1";
const MODAL_SCHEMA_VERSION: &str = "j1m";
const SEPARATOR: char = '.';

/// Discord's custom ID bound.
pub(crate) const MAX_CUSTOM_ID_UNITS: usize = 100;

/// Component `id` of the deliberate input Text Display, so the decoder finds
/// it by identity rather than by position.
pub(crate) const INPUT_COMPONENT_ID: u32 = 1001;

/// Inline budget for the encoded input block (UTF-16 units). Longer blocks go
/// to the readable input attachment so the result keeps room for text and
/// diagnostics under the message budget.
pub(crate) const INLINE_INPUT_BUDGET_UNITS: usize = 1200;

/// Filename of the readable source attachment for long input.
pub(crate) const INPUT_ATTACHMENT_FILENAME: &str = "jbotci-input.txt";

/// Prefix of every header line in the input block (Discord subtext).
const HEADER_PREFIX: &str = "-# ";

// ---------------------------------------------------------------------------
// Digest
// ---------------------------------------------------------------------------

/// First eight bytes of SHA-256 over the canonical source fields.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SourceDigest {
    bytes: [u8; 8],
}

impl SourceDigest {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn of_fields(tool: DiscordTool, fields: &[(SourceField, Option<&str>)]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update([tool.code() as u8]);
        for (field, value) in fields {
            hasher.update([field.digest_tag()]);
            match value {
                Some(value) => {
                    hasher.update([1u8]);
                    hasher.update((value.len() as u64).to_le_bytes());
                    hasher.update(value.as_bytes());
                }
                None => hasher.update([0u8]),
            }
        }
        let hash = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[..8]);
        Self { bytes }
    }

    #[requires(true)]
    #[ensures(ret.len() == 11)]
    pub(crate) fn encode(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.bytes)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn decode(text: &str) -> Option<Self> {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(text)
            .ok()?;
        let bytes: [u8; 8] = decoded.try_into().ok()?;
        Some(Self { bytes })
    }
}

// ---------------------------------------------------------------------------
// Presence mask
// ---------------------------------------------------------------------------

/// Which of a tool's (at most two) source fields are present. A present field
/// with an empty value is not written into the input block, so a cleared
/// optional field costs no visible label; the mask keeps the distinction
/// between "never supplied" and "explicitly cleared".
#[invariant(*bits < 4)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PresenceMask {
    bits: u8,
}

impl PresenceMask {
    #[requires(fields.len() <= 2)]
    #[ensures(true)]
    pub(crate) fn of_fields(fields: &[(SourceField, Option<&str>)]) -> Self {
        let bits = fields
            .iter()
            .enumerate()
            .filter(|(_, (_, value))| value.is_some())
            .fold(0u8, |bits, (index, _)| bits | (1 << index));
        new!(PresenceMask { bits })
    }

    #[requires(index < 2)]
    #[ensures(true)]
    pub(crate) fn is_present(self, index: usize) -> bool {
        self.bits & (1 << index) != 0
    }

    #[requires(true)]
    #[ensures(ret < 4)]
    fn bits(self) -> u8 {
        self.bits
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (bits < 4))]
    fn from_bits(bits: u8) -> Option<Self> {
        try_new!(PresenceMask { bits }).ok()
    }
}

// ---------------------------------------------------------------------------
// Request header (gear button custom_id)
// ---------------------------------------------------------------------------

/// Everything the gear button's custom ID carries.
#[invariant(options_word & !super::request::options_word_mask(*tool) == 0, "options word uses only the tool's bits")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestHeader {
    pub(crate) tool: DiscordTool,
    pub(crate) store: SourceStore,
    pub(crate) presence: PresenceMask,
    pub(crate) options_word: u32,
    pub(crate) page: PageNumber,
    pub(crate) revision: Revision,
    pub(crate) initiator: Snowflake,
    pub(crate) build_tag: BuildTag,
    pub(crate) digest: SourceDigest,
}

impl RequestHeader {
    /// The header describing `published` stored in `store`.
    #[requires(true)]
    #[ensures(ret.tool == published.request.tool() && ret.store == store)]
    #[ensures(ret.revision == published.revision)]
    pub(crate) fn describe(published: &PublishedRequest, store: SourceStore) -> Self {
        let fields = borrowed_fields(&published.request);
        new!(RequestHeader {
            tool: published.request.tool(),
            store,
            presence: PresenceMask::of_fields(&fields),
            options_word: published.request.options_word(),
            page: published.request.page(),
            revision: published.revision,
            initiator: published.initiator.clone(),
            build_tag: published.build_tag.clone(),
            digest: SourceDigest::of_fields(published.request.tool(), &fields),
        })
    }

    /// The custom ID text. Always ASCII and within Discord's 100-character
    /// bound: the worst case is 2+1+3+1+3+1+2+1+10+1+20+1+16+1+11 = 74.
    #[requires(true)]
    #[ensures(ret.is_ascii() && ret.len() <= MAX_CUSTOM_ID_UNITS)]
    pub(crate) fn encode(&self) -> String {
        format!(
            "{SCHEMA_VERSION}{SEPARATOR}{}{}{:x}{SEPARATOR}{:x}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}",
            self.tool.code(),
            self.store.code(),
            self.presence.bits(),
            self.options_word,
            self.page.get(),
            self.revision.get(),
            self.initiator,
            self.build_tag.as_str(),
            self.digest.encode(),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|header| header.encode() == custom_id) || ret.is_err())]
    pub(crate) fn decode(custom_id: &str) -> Result<Self, HeaderDecodeError> {
        let mut parts = custom_id.split(SEPARATOR);
        let schema = parts.next().unwrap_or_default();
        if schema != SCHEMA_VERSION {
            return Err(HeaderDecodeError::UnsupportedSchema {
                found: schema.to_owned(),
            });
        }
        let malformed = |what: &'static str| HeaderDecodeError::Malformed { what };
        let identity = parts.next().ok_or(malformed("tool"))?;
        let mut identity_chars = identity.chars();
        let tool = identity_chars
            .next()
            .and_then(DiscordTool::from_code)
            .ok_or(malformed("tool"))?;
        let store = identity_chars
            .next()
            .and_then(SourceStore::from_code)
            .ok_or(malformed("store"))?;
        let presence = identity_chars
            .next()
            .and_then(|digit| digit.to_digit(16))
            .and_then(|bits| u8::try_from(bits).ok())
            .and_then(PresenceMask::from_bits)
            .ok_or(malformed("presence"))?;
        if identity_chars.next().is_some() {
            return Err(malformed("tool"));
        }
        let options_word = parts
            .next()
            .and_then(|text| u32::from_str_radix(text, 16).ok())
            .ok_or(malformed("options"))?;
        let page = parts
            .next()
            .and_then(|text| text.parse::<u16>().ok())
            .and_then(PageNumber::new)
            .ok_or(malformed("page"))?;
        let revision = parts
            .next()
            .and_then(|text| text.parse::<u32>().ok())
            .map(Revision::new)
            .ok_or(malformed("revision"))?;
        let initiator = parts
            .next()
            .and_then(|text| Snowflake::parse(text).ok())
            .ok_or(malformed("initiator"))?;
        let build_tag = parts
            .next()
            .and_then(|text| BuildTag::parse(text).ok())
            .ok_or(malformed("build"))?;
        let digest = parts
            .next()
            .and_then(SourceDigest::decode)
            .ok_or(malformed("digest"))?;
        if parts.next().is_some() {
            return Err(malformed("trailing data"));
        }
        let header = try_new!(RequestHeader {
            tool,
            store,
            presence,
            options_word,
            page,
            revision,
            initiator,
            build_tag,
            digest,
        })
        .map_err(|_| malformed("options"))?;
        // Canonical form check: a header that does not re-encode to its own
        // text (leading zeros, uppercase hex) came from somewhere else.
        if header.encode() != custom_id {
            return Err(malformed("non-canonical encoding"));
        }
        Ok(header)
    }

    /// Rebuild the published request from this header and the source fields
    /// decoded from the input block, verifying the integrity digest.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|published| published.revision == self.revision) || ret.is_err())]
    pub(crate) fn rebuild(
        &self,
        fields: Vec<(SourceField, Option<String>)>,
    ) -> Result<PublishedRequest, RebuildError> {
        let borrowed = fields
            .iter()
            .map(|(field, value)| (*field, value.as_deref()))
            .collect::<Vec<_>>();
        if SourceDigest::of_fields(self.tool, &borrowed) != self.digest {
            return Err(RebuildError::DigestMismatch);
        }
        let mut typed = Vec::with_capacity(fields.len());
        for (field, value) in fields {
            let value = value
                .map(|value| SourceText::new(&value))
                .transpose()
                .map_err(|error| RebuildError::Oversize {
                    field,
                    units: error.units,
                })?;
            typed.push((field, value));
        }
        let request = DiscordRequest::from_parts(self.tool, self.options_word, self.page, typed)?;
        Ok(PublishedRequest {
            request,
            revision: self.revision,
            initiator: self.initiator.clone(),
            build_tag: self.build_tag.clone(),
        })
    }
}

#[requires(true)]
#[ensures(ret.len() == request.tool().fields().len())]
fn borrowed_fields(request: &DiscordRequest) -> Vec<(SourceField, Option<&str>)> {
    request
        .source_fields()
        .into_iter()
        .map(|(field, value)| (field, value.map(SourceText::as_str)))
        .collect()
}

#[invariant(::UnsupportedSchema { .. } => true)]
#[invariant(::Malformed { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeaderDecodeError {
    UnsupportedSchema { found: String },
    Malformed { what: &'static str },
}

impl fmt::Display for HeaderDecodeError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { found } => write!(
                formatter,
                "this result was produced by an incompatible jbotci version (state schema `{found}`); rerun the command"
            ),
            Self::Malformed { what } => {
                write!(
                    formatter,
                    "the result's state is malformed ({what}); rerun the command"
                )
            }
        }
    }
}

impl std::error::Error for HeaderDecodeError {}

#[invariant(::Oversize { .. } => true)]
#[invariant(::State(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RebuildError {
    /// The stored source does not match the digest recorded when it was
    /// published: the transport altered it.
    DigestMismatch,
    Oversize {
        field: SourceField,
        units: usize,
    },
    State(RequestStateError),
}

impl From<RequestStateError> for RebuildError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: RequestStateError) -> Self {
        Self::State(error)
    }
}

impl fmt::Display for RebuildError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DigestMismatch => formatter.write_str(
                "the stored input no longer matches what was published, so it cannot be reused; rerun the command",
            ),
            Self::Oversize { field, units } => write!(
                formatter,
                "the stored {} field ({units} characters) exceeds the limit; rerun the command",
                field.label()
            ),
            Self::State(error) => write!(formatter, "{error}; rerun the command"),
        }
    }
}

impl std::error::Error for RebuildError {}

// ---------------------------------------------------------------------------
// Modal header (modal custom_id)
// ---------------------------------------------------------------------------

/// What the customization modal's custom ID carries: enough to tie a
/// submission to the message revision it was opened from and to the
/// initiator policy. Form values arrive in the submission itself and the
/// target message arrives as `interaction.message`.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModalHeader {
    pub(crate) tool: DiscordTool,
    pub(crate) revision: Revision,
    pub(crate) initiator: Snowflake,
}

impl ModalHeader {
    #[requires(true)]
    #[ensures(ret.is_ascii() && ret.len() <= MAX_CUSTOM_ID_UNITS)]
    pub(crate) fn encode(&self) -> String {
        format!(
            "{MODAL_SCHEMA_VERSION}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}",
            self.tool.code(),
            self.revision.get(),
            self.initiator,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|header| header.encode() == custom_id) || ret.is_err())]
    pub(crate) fn decode(custom_id: &str) -> Result<Self, HeaderDecodeError> {
        let mut parts = custom_id.split(SEPARATOR);
        let schema = parts.next().unwrap_or_default();
        if schema != MODAL_SCHEMA_VERSION {
            return Err(HeaderDecodeError::UnsupportedSchema {
                found: schema.to_owned(),
            });
        }
        let malformed = |what: &'static str| HeaderDecodeError::Malformed { what };
        let tool = parts
            .next()
            .and_then(|text| {
                let mut chars = text.chars();
                let tool = chars.next().and_then(DiscordTool::from_code)?;
                chars.next().is_none().then_some(tool)
            })
            .ok_or(malformed("tool"))?;
        let revision = parts
            .next()
            .and_then(|text| text.parse::<u32>().ok())
            .map(Revision::new)
            .ok_or(malformed("revision"))?;
        let initiator = parts
            .next()
            .and_then(|text| Snowflake::parse(text).ok())
            .ok_or(malformed("initiator"))?;
        if parts.next().is_some() {
            return Err(malformed("trailing data"));
        }
        let header = Self {
            tool,
            revision,
            initiator,
        };
        if header.encode() != custom_id {
            return Err(malformed("non-canonical encoding"));
        }
        Ok(header)
    }
}

// ---------------------------------------------------------------------------
// Input block
// ---------------------------------------------------------------------------

/// Encode the source fields as the input block.
///
/// Layout (every line ends the previous one with `\n`; the block has no
/// trailing newline):
///
/// ```text
/// <primary value lines, escaped>          only when the primary field is non-empty
/// -# <secondary label>                    only for non-empty secondary fields
/// <secondary value lines, escaped>
/// -# <status>                             terminator; `status` starts with the tool name
/// ```
///
/// Because `-` is always escaped inside values, a value line can never start
/// with the header prefix, so the block parses unambiguously for any input,
/// including empty lines, a final newline and header look-alike text. Which
/// fields are present (as opposed to non-empty) is carried by the header's
/// presence mask, not by the block.
#[requires(fields.len() == tool.fields().len())]
#[requires(fields.iter().zip(tool.fields()).all(|((field, _), expected)| field == expected))]
#[requires(status.starts_with(tool.name()) && !status.contains('\n') && !status.contains('\r'))]
#[ensures(ret.ends_with(status) && !ret.ends_with('\n'))]
pub(crate) fn encode_input_block(
    tool: DiscordTool,
    fields: &[(SourceField, Option<&str>)],
    status: &str,
) -> String {
    let mut block = String::new();
    for (index, (field, value)) in fields.iter().enumerate() {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            continue;
        };
        if index > 0 {
            block.push_str(HEADER_PREFIX);
            block.push_str(field.label());
            block.push('\n');
        }
        for line in value.split('\n') {
            block.push_str(&escape_value_line(line));
            block.push('\n');
        }
    }
    block.push_str(HEADER_PREFIX);
    block.push_str(status);
    block
}

/// Decode an input block produced by [`encode_input_block`] for `tool`.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|fields| fields.len() == tool.fields().len()) || ret.is_err())]
pub(crate) fn decode_input_block(
    tool: DiscordTool,
    presence: PresenceMask,
    block: &str,
) -> Result<Vec<(SourceField, Option<String>)>, InputBlockError> {
    let layout = tool.fields();
    let mut values: Vec<Option<String>> = vec![None; layout.len()];
    let mut current: Option<usize> = Some(0);
    let mut lines: Vec<&str> = Vec::new();
    let mut terminated = false;
    for line in block.split('\n') {
        if let Some(label) = line.strip_prefix(HEADER_PREFIX) {
            flush_field_lines(layout, presence, current, &mut lines, &mut values)?;
            if label.starts_with(tool.name()) {
                terminated = true;
                current = None;
                continue;
            }
            if terminated {
                return Err(InputBlockError::Malformed {
                    what: "content after the terminator",
                });
            }
            let field = SourceField::from_label(label).ok_or(InputBlockError::Malformed {
                what: "unknown field header",
            })?;
            let index = layout
                .iter()
                .position(|candidate| *candidate == field)
                .ok_or(InputBlockError::UnexpectedField { field })?;
            if index == 0 {
                return Err(InputBlockError::Malformed {
                    what: "primary field carries a header",
                });
            }
            if values[index].is_some() {
                return Err(InputBlockError::Malformed {
                    what: "duplicate field header",
                });
            }
            current = Some(index);
            continue;
        }
        if terminated {
            return Err(InputBlockError::Malformed {
                what: "content after the terminator",
            });
        }
        lines.push(line);
    }
    if !terminated {
        return Err(InputBlockError::Malformed {
            what: "missing terminator",
        });
    }
    let mut fields = Vec::with_capacity(layout.len());
    for (index, field) in layout.iter().enumerate() {
        let value = match values[index].take() {
            Some(value) => Some(value),
            None if presence.is_present(index) => Some(String::new()),
            None => None,
        };
        fields.push((*field, value));
    }
    Ok(fields)
}

#[invariant(::Malformed { .. } => true)]
#[invariant(::UnexpectedField { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputBlockError {
    Malformed { what: &'static str },
    UnexpectedField { field: SourceField },
    BadEscape,
}

impl fmt::Display for InputBlockError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { what } => write!(formatter, "stored input is malformed ({what})"),
            Self::UnexpectedField { field } => write!(
                formatter,
                "stored input carries a {} field the state does not declare",
                field.label()
            ),
            Self::BadEscape => formatter.write_str("stored input carries an invalid escape"),
        }
    }
}

impl std::error::Error for InputBlockError {}

/// Store the value lines collected for `current` into `values`.
#[requires(values.len() == layout.len())]
#[requires(current.is_none_or(|index| index < layout.len()))]
#[ensures(lines.is_empty())]
fn flush_field_lines(
    layout: &[SourceField],
    presence: PresenceMask,
    current: Option<usize>,
    lines: &mut Vec<&str>,
    values: &mut [Option<String>],
) -> Result<(), InputBlockError> {
    let result = match current {
        Some(index) if !lines.is_empty() => {
            if !presence.is_present(index) {
                Err(InputBlockError::UnexpectedField {
                    field: layout[index],
                })
            } else {
                let mut value = String::new();
                let mut error = None;
                for line in lines.iter() {
                    match unescape_value_line(line) {
                        Ok(unescaped) => {
                            value.push_str(&unescaped);
                            value.push('\n');
                        }
                        Err(failure) => {
                            error = Some(failure);
                            break;
                        }
                    }
                }
                match error {
                    Some(error) => Err(error),
                    None => {
                        value.pop();
                        values[index] = Some(value);
                        Ok(())
                    }
                }
            }
        }
        Some(_) => Ok(()),
        None if lines.is_empty() => Ok(()),
        None => Err(InputBlockError::Malformed {
            what: "value lines without a field",
        }),
    };
    lines.clear();
    result
}

/// Escape one line of a source value for a Text Display. This is the
/// workspace's Discord text escape, so a value renders exactly as typed, can
/// never look like a header line, and decodes back to itself.
#[requires(!line.contains('\n'))]
#[ensures(!ret.starts_with(HEADER_PREFIX) && !ret.starts_with('-'))]
#[ensures(unescape_value_line(&ret).as_deref() == Ok(line))]
pub(crate) fn escape_value_line(line: &str) -> String {
    escape_discord_markdown_line(line)
}

/// Reverse [`escape_value_line`]. Any `\` before ASCII punctuation is removed;
/// a `\` before anything else, or a trailing one, was not produced by the
/// encoder.
#[requires(true)]
#[ensures(true)]
pub(crate) fn unescape_value_line(line: &str) -> Result<String, InputBlockError> {
    let mut value = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            match chars.next() {
                Some(next) if next.is_ascii_punctuation() => value.push(next),
                _ => return Err(InputBlockError::BadEscape),
            }
        } else {
            value.push(character);
        }
    }
    Ok(value)
}

/// Whether an encoded input block fits the inline component budget.
#[requires(true)]
#[ensures(ret == (utf16_len(block) <= INLINE_INPUT_BUDGET_UNITS))]
pub(crate) fn input_block_fits_inline(block: &str) -> bool {
    utf16_len(block) <= INLINE_INPUT_BUDGET_UNITS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::{
        CollisionScope, CuktaMode, CuktaOptions, CuktaRequest, CuktaResultKind, CuktaResultKindSet,
        GentufaOptions, GentufaRequest, GentufaTextView, GimfihiOptions, GimfihiPreset,
        GimfihiRequest, GismuShape, GismuShapeSet, JvozbaOptions, JvozbaRequest, JvozbaTarget,
        MAX_SOURCE_UNITS, VlackuMode, VlackuOptions, VlackuRequest, VlackuWordType,
        VlackuWordTypeSet, VlaseiOptions, VlaseiRequest, VlaseiView, VlataiOptions, VlataiRequest,
    };

    #[requires(true)]
    #[ensures(true)]
    fn text(value: &str) -> SourceText {
        SourceText::new(value).expect("test text fits")
    }

    #[requires(true)]
    #[ensures(true)]
    fn initiator() -> Snowflake {
        Snowflake::parse("123456789012345678").expect("snowflake")
    }

    #[requires(true)]
    #[ensures(true)]
    fn publish(request: DiscordRequest, revision: u32) -> PublishedRequest {
        PublishedRequest {
            request,
            revision: Revision::new(revision),
            initiator: initiator(),
            build_tag: BuildTag::parse("50c513f").expect("tag"),
        }
    }

    /// Round-trip one published request through the header and the block in
    /// both stores.
    #[requires(true)]
    #[ensures(true)]
    fn assert_round_trip(published: &PublishedRequest) {
        for store in [SourceStore::Inline, SourceStore::Attachment] {
            let header = RequestHeader::describe(published, store);
            let custom_id = header.encode();
            assert!(custom_id.len() <= MAX_CUSTOM_ID_UNITS, "{custom_id}");
            let decoded = RequestHeader::decode(&custom_id).expect("header decodes");
            assert_eq!(decoded, header);
            let fields = borrowed_fields(&published.request);
            let tool = published.request.tool();
            let status = format!("{tool} · test");
            let block = encode_input_block(tool, &fields, &status);
            let decoded_fields =
                decode_input_block(tool, decoded.presence, &block).expect("block decodes");
            let rebuilt = decoded.rebuild(decoded_fields).expect("request rebuilds");
            assert_eq!(&rebuilt, published, "store={store:?}\n{block}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_message_from_before_the_ordered_parts_syntax_reopens_as_the_same_build() {
        // The wire state exactly as the previous version published it: two
        // fields, the words and a whitespace-separated fixed rafsi list, with
        // the digest taken over both. Nothing here is a reconstruction of the
        // new format; it is what a message in a channel right now holds.
        let tool = DiscordTool::Jvozba;
        let legacy_fields: Vec<(SourceField, Option<&str>)> = vec![
            (SourceField::Parts, Some("klama bajra")),
            (SourceField::FixedRafsi, Some("kla bar")),
        ];
        let status = format!("{tool} · test");
        let block = encode_input_block(tool, &legacy_fields, &status);
        let presence = PresenceMask::of_fields(&legacy_fields);
        let digest = SourceDigest::of_fields(tool, &legacy_fields);
        let header = new!(RequestHeader {
            tool,
            store: SourceStore::Inline,
            presence,
            options_word: 0,
            page: PageNumber::first(),
            revision: Revision::INITIAL,
            initiator: Snowflake::parse("123456789012345678").expect("snowflake"),
            build_tag: BuildTag::parse("older").expect("tag"),
            digest,
        });

        let decoded_fields =
            decode_input_block(tool, header.presence, &block).expect("the old block decodes");
        let rebuilt = header
            .rebuild(decoded_fields)
            .expect("the old state rebuilds");
        let DiscordRequest::Jvozba(request) = &rebuilt.request else {
            panic!("a jvozba request");
        };
        // The old builder put every fixed rafsi after all of the words, so
        // this is the same sequence that message would have built.
        assert_eq!(request.parts.as_str(), "klama bajra -kla- -bar-");

        // And what it rebuilds to now publishes and reopens unchanged.
        assert_round_trip(&rebuilt);
    }

    /// Every awkward value the PM asked the codec to prove.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn awkward_values() -> Vec<&'static str> {
        vec![
            "",
            " ",
            "  leading and trailing  ",
            "\n",
            "a\n",
            "\na",
            "a\n\nb",
            "a\r\nb\r\n",
            "\r",
            "back\\slash \\\\ \\-# text",
            "-# text",
            "-# dialect",
            "-# gentufa · status",
            "\\-\\# already escaped *bold* _em_ ~~x~~ `code` ||spoiler|| > quote # heading",
            "1. ordered\n2. list\n+ plus\n- dash\n* star",
            "[link](https://example.com) <@123> @everyone :smile: <t:0>",
            "mi klama .i do klama .i ko'a",
            "😀 supplementary 𝔘𝔫𝔦𝔠𝔬𝔡𝔢 \u{1F600}\u{1F3FB} 日本語 ĉ",
            "tab\there",
        ]
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_tool_round_trips_all_two_field_combinations_of_awkward_values() {
        let values = awkward_values();
        let mut count = 0;
        for tool in DiscordTool::ALL {
            let fields = tool.fields();
            let primaries = values.iter().map(|value| Some(*value)).collect::<Vec<_>>();
            let secondaries = if fields.len() == 2 {
                let mut options = vec![None];
                options.extend(values.iter().map(|value| Some(*value)));
                options
            } else {
                vec![None]
            };
            for primary in &primaries {
                for secondary in &secondaries {
                    let mut assigned = vec![(fields[0], *primary)];
                    if fields.len() == 2 {
                        assigned.push((fields[1], *secondary));
                    }
                    // Tools whose primary field is required cannot be rebuilt
                    // with an absent primary; the codec still handles the
                    // present-but-empty case for every tool.
                    let typed = assigned
                        .iter()
                        .map(|(field, value)| (*field, value.map(text)))
                        .collect::<Vec<_>>();
                    let Ok(request) =
                        DiscordRequest::from_parts(tool, 0, PageNumber::first(), typed)
                    else {
                        continue;
                    };
                    let published = publish(request, 3);
                    assert_round_trip(&published);
                    count += 1;
                }
            }
            // Absent optional primary/secondary fields (cukta contents,
            // gimfihi without sources, missing dialect/rafsi).
            if tool == DiscordTool::Cukta {
                let request = DiscordRequest::Cukta(CuktaRequest {
                    query: None,
                    options: CuktaOptions {
                        mode: CuktaMode::Contents,
                        kinds: CuktaResultKindSet::empty(),
                        page: PageNumber::first(),
                    },
                });
                assert_round_trip(&publish(request, 1));
            }
            if tool == DiscordTool::Gimfihi {
                assert_round_trip(&publish(
                    DiscordRequest::Gimfihi(GimfihiRequest {
                        sources: None,
                        options: GimfihiOptions::default(),
                    }),
                    1,
                ));
            }
        }
        assert!(count > 100, "{count} combinations exercised");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn absent_and_explicitly_empty_fields_are_distinguished_without_labels() {
        let absent = publish(
            DiscordRequest::Gentufa(GentufaRequest {
                text: text("mi klama"),
                dialect: None,
                options: GentufaOptions::default(),
            }),
            1,
        );
        let cleared = publish(
            DiscordRequest::Gentufa(GentufaRequest {
                text: text("mi klama"),
                dialect: Some(text("")),
                options: GentufaOptions::default(),
            }),
            1,
        );
        let block_absent = encode_input_block(
            DiscordTool::Gentufa,
            &borrowed_fields(&absent.request),
            "gentufa · brackets",
        );
        let block_cleared = encode_input_block(
            DiscordTool::Gentufa,
            &borrowed_fields(&cleared.request),
            "gentufa · brackets",
        );
        assert_eq!(block_absent, "mi klama\n-# gentufa · brackets");
        assert_eq!(block_cleared, block_absent, "a cleared field adds no label");
        let header_absent = RequestHeader::describe(&absent, SourceStore::Inline);
        let header_cleared = RequestHeader::describe(&cleared, SourceStore::Inline);
        assert_ne!(header_absent.presence, header_cleared.presence);
        assert_ne!(header_absent.digest, header_cleared.digest);
        assert_round_trip(&absent);
        assert_round_trip(&cleared);
        // A present-but-empty required primary field also needs no label.
        let empty_primary = publish(
            DiscordRequest::Gentufa(GentufaRequest {
                text: text(""),
                dialect: Some(text("zantufa")),
                options: GentufaOptions::default(),
            }),
            2,
        );
        let block = encode_input_block(
            DiscordTool::Gentufa,
            &borrowed_fields(&empty_primary.request),
            "gentufa",
        );
        assert_eq!(block, "-# dialect\nzantufa\n-# gentufa");
        assert_round_trip(&empty_primary);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nondefault_options_and_pages_round_trip_for_every_tool() {
        let requests = vec![
            DiscordRequest::Gentufa(GentufaRequest {
                text: text("mi pa moi klama"),
                dialect: Some(text("(cbm ce-ki-tau)")),
                options: GentufaOptions {
                    view: GentufaTextView::Tree,
                    include_diagram: true,
                    show_elided: true,
                    show_compounds: false,
                    show_glosses: true,
                },
            }),
            DiscordRequest::Vlasei(VlaseiRequest {
                text: text("coi"),
                dialect: None,
                options: VlaseiOptions {
                    view: VlaseiView::Tree,
                    decompose_lujvo: true,
                },
            }),
            DiscordRequest::Vlatai(VlataiRequest {
                text: text("klama"),
                dialect: Some(text("")),
                options: VlataiOptions {
                    mark_stress: false,
                    mark_glides: false,
                    show_details: false,
                },
            }),
            DiscordRequest::Vlacku(VlackuRequest {
                query: text("/^kla/"),
                options: VlackuOptions {
                    mode: VlackuMode::Sound,
                    word_types: VlackuWordTypeSet::empty()
                        .with(VlackuWordType::Cmavo)
                        .with(VlackuWordType::Cmevla),
                    decompose_lujvo: false,
                    show_etymology: true,
                    page: PageNumber::new(400).expect("page"),
                },
            }),
            DiscordRequest::Cukta(CuktaRequest {
                query: Some(text("6.8")),
                options: CuktaOptions {
                    mode: CuktaMode::Example,
                    kinds: CuktaResultKindSet::empty()
                        .with(CuktaResultKind::Section)
                        .with(CuktaResultKind::Paragraph),
                    page: PageNumber::new(25).expect("page"),
                },
            }),
            DiscordRequest::Jvozba(JvozbaRequest {
                parts: text("klama bajra"),
                options: JvozbaOptions {
                    target: JvozbaTarget::Cmevla,
                },
            }),
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: Some(text("eng:go\nspa:[ir]; cmn:cu")),
                options: GimfihiOptions {
                    preset: Some(GimfihiPreset::Data1999),
                    shapes: GismuShapeSet::empty().with(GismuShape::Ccvcv),
                    collisions: CollisionScope::Official,
                    show_collisions: true,
                    all_letters: false,
                    require_free_short_rafsi: true,
                    page: PageNumber::new(12).expect("page"),
                },
            }),
        ];
        for request in requests {
            assert_round_trip(&publish(request, u32::MAX - 1));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn header_stays_within_the_custom_id_bound_at_its_worst_case() {
        let published = PublishedRequest {
            request: DiscordRequest::Gimfihi(GimfihiRequest {
                sources: Some(text("eng:go")),
                options: GimfihiOptions {
                    preset: Some(GimfihiPreset::Ilmen12),
                    shapes: GismuShapeSet::empty()
                        .with(GismuShape::Ccvcv)
                        .with(GismuShape::Cvccv),
                    collisions: CollisionScope::None,
                    show_collisions: true,
                    all_letters: true,
                    require_free_short_rafsi: true,
                    page: PageNumber::new(25).expect("page"),
                },
            }),
            revision: Revision::new(u32::MAX),
            initiator: Snowflake::parse("99999999999999999999").expect("20-digit snowflake"),
            build_tag: BuildTag::parse("abcdefghij012345").expect("16-char tag"),
        };
        let custom_id = RequestHeader::describe(&published, SourceStore::Attachment).encode();
        assert!(
            custom_id.len() <= MAX_CUSTOM_ID_UNITS,
            "{}: {custom_id}",
            custom_id.len()
        );
        assert_eq!(custom_id.len(), 74);
        let modal = ModalHeader {
            tool: DiscordTool::Gimfihi,
            revision: Revision::new(u32::MAX),
            initiator: published.initiator.clone(),
        };
        let modal_id = modal.encode();
        assert!(modal_id.len() <= MAX_CUSTOM_ID_UNITS);
        assert_eq!(ModalHeader::decode(&modal_id).expect("modal header"), modal);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn foreign_and_malformed_headers_are_rejected() {
        let published = publish(
            DiscordRequest::Vlasei(VlaseiRequest {
                text: text("coi"),
                dialect: None,
                options: VlaseiOptions::default(),
            }),
            1,
        );
        let good = RequestHeader::describe(&published, SourceStore::Inline).encode();
        assert_eq!(
            RequestHeader::decode("j0.vi1.0.1.1.1.dev.AAAAAAAAAAA"),
            Err(HeaderDecodeError::UnsupportedSchema {
                found: "j0".to_owned()
            })
        );
        assert_eq!(
            RequestHeader::decode("legacy-button"),
            Err(HeaderDecodeError::UnsupportedSchema {
                found: "legacy-button".to_owned()
            })
        );
        for (mutated, what) in [
            (good.replacen(".vi", ".xi", 1), "tool"),
            (good.replacen(".vi", ".vz", 1), "store"),
            (good.replacen(".vi1.", ".vi9.", 1), "presence"),
            (format!("{good}.extra"), "trailing data"),
            (
                good.replacen(".1.1.", ".01.1.", 1),
                "non-canonical encoding",
            ),
        ] {
            let error = RequestHeader::decode(&mutated).expect_err(what);
            assert!(
                matches!(error, HeaderDecodeError::Malformed { .. }),
                "{mutated}: {error}"
            );
        }
        // Page numbers are no longer bounded by a selector, so a deep page
        // decodes; only a page that is not a positive number is malformed.
        let deep = good.replacen(".1.1.", ".2600.1.", 1);
        assert!(
            RequestHeader::decode(&deep).is_ok_and(|header| header.page.get() == 2600),
            "{deep}"
        );
        let zero = good.replacen(".1.1.", ".0.1.", 1);
        assert!(matches!(
            RequestHeader::decode(&zero),
            Err(HeaderDecodeError::Malformed { what: "page" })
        ));
        assert_eq!(
            ModalHeader::decode("j1.v.1.1"),
            Err(HeaderDecodeError::UnsupportedSchema {
                found: "j1".to_owned()
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn digest_mismatch_and_undeclared_fields_are_errors_not_guesses() {
        let published = publish(
            DiscordRequest::Gentufa(GentufaRequest {
                text: text("mi klama"),
                dialect: None,
                options: GentufaOptions::default(),
            }),
            1,
        );
        let header = RequestHeader::describe(&published, SourceStore::Inline);
        // Transport mutated the stored text.
        let mutated = decode_input_block(
            DiscordTool::Gentufa,
            header.presence,
            "mi klama\u{200b}\n-# gentufa",
        )
        .expect("block decodes");
        assert_eq!(header.rebuild(mutated), Err(RebuildError::DigestMismatch));
        // The block carries a dialect the presence mask does not declare.
        assert_eq!(
            decode_input_block(
                DiscordTool::Gentufa,
                header.presence,
                "mi klama\n-# dialect\nx\n-# gentufa"
            ),
            Err(InputBlockError::UnexpectedField {
                field: SourceField::Dialect
            })
        );
        // Unknown header, missing terminator, content after the terminator,
        // a primary field header, a bad escape.
        for (block, what) in [
            ("mi klama\n-# bogus\n-# gentufa", "unknown field header"),
            ("mi klama", "missing terminator"),
            ("mi klama\n-# gentufa\nmore", "content after the terminator"),
            (
                "-# text\nmi klama\n-# gentufa",
                "primary field carries a header",
            ),
        ] {
            assert_eq!(
                decode_input_block(DiscordTool::Gentufa, header.presence, block),
                Err(InputBlockError::Malformed { what }),
                "{block}"
            );
        }
        assert_eq!(
            decode_input_block(
                DiscordTool::Gentufa,
                header.presence,
                "mi \\klama\n-# gentufa"
            ),
            Err(InputBlockError::BadEscape)
        );
        assert_eq!(
            decode_input_block(
                DiscordTool::Gentufa,
                header.presence,
                "mi klama\\\n-# gentufa"
            ),
            Err(InputBlockError::BadEscape)
        );
        // A block for another tool's field layout fails at rebuild.
        let vlacku_header = RequestHeader::describe(
            &publish(
                DiscordRequest::Vlacku(VlackuRequest {
                    query: text("klama"),
                    options: VlackuOptions::default(),
                }),
                1,
            ),
            SourceStore::Inline,
        );
        let wrong_fields = vec![(SourceField::Text, Some("klama".to_owned()))];
        assert!(matches!(
            vlacku_header.rebuild(wrong_fields),
            Err(RebuildError::DigestMismatch)
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn escaping_neutralizes_markdown_and_header_syntax() {
        assert_eq!(escape_value_line("-# text"), "\\-# text");
        assert_eq!(escape_value_line("1. one"), "1\\. one");
        assert_eq!(escape_value_line("1.5 not a list"), "1\\.5 not a list");
        assert_eq!(escape_value_line("v1.5 not a list"), "v1.5 not a list");
        assert_eq!(escape_value_line("+ plus"), "\\+ plus");
        assert_eq!(escape_value_line("a + b"), "a + b");
        assert_eq!(escape_value_line("mi klama .i do"), "mi klama .i do");
        assert_eq!(
            escape_value_line("<@123> :x: [a](b)"),
            "\\<\\@123\\> \\:x\\: \\[a\\]\\(b\\)"
        );
        for value in awkward_values() {
            for line in value.split('\n') {
                let escaped = escape_value_line(line);
                assert!(!escaped.starts_with(HEADER_PREFIX));
                assert_eq!(unescape_value_line(&escaped).expect("unescapes"), line);
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inline_budget_decision_uses_the_escaped_block() {
        let short = encode_input_block(
            DiscordTool::Gentufa,
            &[
                (SourceField::Text, Some("mi klama")),
                (SourceField::Dialect, None),
            ],
            "gentufa",
        );
        assert!(input_block_fits_inline(&short));
        let long_source = "*".repeat(700);
        let long = encode_input_block(
            DiscordTool::Gentufa,
            &[
                (SourceField::Text, Some(&long_source)),
                (SourceField::Dialect, None),
            ],
            "gentufa",
        );
        assert!(
            !input_block_fits_inline(&long),
            "700 stars escape to 1400 units"
        );
        let maximal = "a".repeat(MAX_SOURCE_UNITS);
        let block = encode_input_block(
            DiscordTool::Gentufa,
            &[
                (SourceField::Text, Some(&maximal)),
                (SourceField::Dialect, Some(&maximal)),
            ],
            "gentufa",
        );
        assert!(!input_block_fits_inline(&block));
        let published = publish(
            DiscordRequest::Gentufa(GentufaRequest {
                text: text(&maximal),
                dialect: Some(text(&maximal)),
                options: GentufaOptions::default(),
            }),
            1,
        );
        assert_round_trip(&published);
    }
}

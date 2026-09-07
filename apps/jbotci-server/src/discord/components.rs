//! Typed Discord Components V2, modal and interaction-response models.
//!
//! Every message jbotci publishes is a component-driven message
//! (`IS_COMPONENTS_V2`): one Section whose accessory is the single gear button,
//! Text Displays for the result and diagnostics, an optional Media Gallery for
//! the diagram and File components for the readable input/overflow
//! attachments. The types here carry the platform bounds as invariants and
//! serialize to the exact JSON Discord expects, so a payload that exists is a
//! payload Discord will accept as far as its documented limits go.
//!
//! The documented application budget for rendered text is
//! [`MESSAGE_TEXT_BUDGET_UNITS`] UTF-16 units across every Text Display in the
//! message, including the Section's children; presenters must fit under it or
//! move content to an attachment before a payload is built.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires, try_new};
use serde_json::{Value, json};
use vec1::Vec1;

use super::codec::MAX_CUSTOM_ID_UNITS;
use super::request::{Snowflake, utf16_len};

/// `IS_COMPONENTS_V2` message flag.
pub(crate) const FLAG_IS_COMPONENTS_V2: u64 = 1 << 15;
/// `EPHEMERAL` message flag.
pub(crate) const FLAG_EPHEMERAL: u64 = 1 << 6;

/// Application budget for rendered text per message (UTF-16 units) across all
/// Text Displays. Documented product bound (#893 review), independent of any
/// larger platform allowance.
pub(crate) const MESSAGE_TEXT_BUDGET_UNITS: usize = 4000;
/// Per-component Text Display cap (platform).
pub(crate) const TEXT_DISPLAY_MAX_UNITS: usize = 4000;
/// The stricter of the documented component-count limits (the V2 launch note
/// says 10 top-level / 30 total; the current reference says 40 total).
pub(crate) const MAX_TOP_LEVEL_COMPONENTS: usize = 10;
pub(crate) const MAX_TOTAL_COMPONENTS: usize = 30;
pub(crate) const MAX_SECTION_TEXTS: usize = 3;
pub(crate) const MAX_GALLERY_ITEMS: usize = 10;
pub(crate) const MAX_ATTACHMENT_DESCRIPTION_UNITS: usize = 1024;
pub(crate) const MAX_BUTTON_LABEL_UNITS: usize = 80;

pub(crate) const MAX_MODAL_TITLE_UNITS: usize = 45;
pub(crate) const MAX_MODAL_COMPONENTS: usize = 5;
pub(crate) const MAX_LABEL_UNITS: usize = 45;
pub(crate) const MAX_LABEL_DESCRIPTION_UNITS: usize = 100;
pub(crate) const MAX_TEXT_INPUT_UNITS: usize = 4000;
pub(crate) const MAX_PLACEHOLDER_UNITS: usize = 100;
pub(crate) const MAX_SELECT_PLACEHOLDER_UNITS: usize = 150;
pub(crate) const MAX_SELECT_OPTIONS: usize = 25;
pub(crate) const MAX_OPTION_TEXT_UNITS: usize = 100;
pub(crate) const MIN_RADIO_OPTIONS: usize = 2;
pub(crate) const MAX_GROUP_OPTIONS: usize = 10;

// Discord component type numbers.
const TYPE_BUTTON: u8 = 2;
const TYPE_STRING_SELECT: u8 = 3;
const TYPE_TEXT_INPUT: u8 = 4;
const TYPE_SECTION: u8 = 9;
const TYPE_TEXT_DISPLAY: u8 = 10;
const TYPE_MEDIA_GALLERY: u8 = 12;
const TYPE_FILE: u8 = 13;
const TYPE_LABEL: u8 = 18;
const TYPE_RADIO_GROUP: u8 = 21;
const TYPE_CHECKBOX_GROUP: u8 = 22;

// Interaction callback types.
const CALLBACK_PONG: u8 = 1;
const CALLBACK_CHANNEL_MESSAGE: u8 = 4;
const CALLBACK_DEFERRED_CHANNEL_MESSAGE: u8 = 5;
const CALLBACK_DEFERRED_UPDATE_MESSAGE: u8 = 6;
const CALLBACK_MODAL: u8 = 9;

// ---------------------------------------------------------------------------
// Leaf values
// ---------------------------------------------------------------------------

/// A component or modal custom ID within Discord's bound.
#[invariant(!value.is_empty() && utf16_len(value) <= MAX_CUSTOM_ID_UNITS)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CustomId {
    value: String,
}

impl CustomId {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.as_str() == value) || ret.is_err())]
    pub(crate) fn new(value: &str) -> Result<Self, BoundsError> {
        try_new!(CustomId {
            value: value.to_owned()
        })
        .map_err(|_| BoundsError::new("custom_id", utf16_len(value), MAX_CUSTOM_ID_UNITS))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

/// Filename of an attachment referenced as `attachment://<name>`.
#[invariant(!name.is_empty() && name.is_ascii() && !name.contains('/') && !name.contains('\\') && !name.contains(char::is_whitespace))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AttachmentName {
    name: String,
}

impl AttachmentName {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|attachment| attachment.as_str() == name) || ret.is_err())]
    pub(crate) fn new(name: &str) -> Result<Self, InvalidAttachmentName> {
        try_new!(AttachmentName {
            name: name.to_owned()
        })
        .map_err(|_| InvalidAttachmentName {
            name: name.to_owned(),
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.name
    }

    /// The `attachment://` reference used inside components.
    #[requires(true)]
    #[ensures(ret.starts_with("attachment://"))]
    pub(crate) fn reference(&self) -> String {
        format!("attachment://{}", self.name)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidAttachmentName {
    pub(crate) name: String,
}

impl fmt::Display for InvalidAttachmentName {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`{}` is not a usable attachment name", self.name)
    }
}

/// A text value that exceeded a platform or application bound.
#[invariant(*units > *limit)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundsError {
    pub(crate) what: &'static str,
    pub(crate) units: usize,
    pub(crate) limit: usize,
}

impl BoundsError {
    #[requires(units > limit)]
    #[ensures(ret.units == units && ret.limit == limit)]
    pub(crate) fn new(what: &'static str, units: usize, limit: usize) -> Self {
        new!(BoundsError { what, units, limit })
    }
}

impl fmt::Display for BoundsError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} is {} characters, more than the {}-character limit",
            self.what, self.units, self.limit
        )
    }
}

impl std::error::Error for BoundsError {}

// ---------------------------------------------------------------------------
// Message components
// ---------------------------------------------------------------------------

/// Markdown text component. `id` is Discord's optional 32-bit component
/// identifier, used to find the deliberate input component on the way back.
#[invariant(!content.is_empty() && utf16_len(content) <= TEXT_DISPLAY_MAX_UNITS)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextDisplay {
    pub(crate) id: Option<u32>,
    pub(crate) content: String,
}

impl TextDisplay {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| text.id == id) || ret.is_err())]
    pub(crate) fn new(id: Option<u32>, content: String) -> Result<Self, BoundsError> {
        if content.is_empty() {
            // Discord rejects an empty Text Display; report it as a zero-length
            // bound violation so callers cannot publish a blank component.
            return Err(new!(BoundsError {
                what: "empty text display",
                units: 1,
                limit: 0,
            }));
        }
        let units = utf16_len(&content);
        try_new!(TextDisplay { id, content })
            .map_err(|_| BoundsError::new("text display", units, TEXT_DISPLAY_MAX_UNITS))
    }

    #[requires(true)]
    #[ensures(ret <= TEXT_DISPLAY_MAX_UNITS)]
    pub(crate) fn units(&self) -> usize {
        utf16_len(&self.content)
    }

    #[requires(true)]
    #[ensures(ret.get("type") == Some(&json!(TYPE_TEXT_DISPLAY)))]
    fn to_json(&self) -> Value {
        let mut value = json!({ "type": TYPE_TEXT_DISPLAY, "content": self.content });
        if let Some(id) = self.id {
            value["id"] = json!(id);
        }
        value
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonStyle {
    Primary,
    Secondary,
    Success,
    Danger,
}

impl ButtonStyle {
    #[requires(true)]
    #[ensures(ret >= 1 && ret <= 4)]
    const fn code(self) -> u8 {
        match self {
            Self::Primary => 1,
            Self::Secondary => 2,
            Self::Success => 3,
            Self::Danger => 4,
        }
    }
}

/// A non-link button. The gear accessory is emoji-only; a label is optional
/// and bounded.
#[invariant(label.is_some() || emoji.is_some(), "a button shows an emoji, a label or both")]
#[invariant(label.as_ref().is_none_or(|label| !label.is_empty() && utf16_len(label) <= MAX_BUTTON_LABEL_UNITS))]
#[invariant(emoji.as_ref().is_none_or(|emoji| !emoji.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Button {
    pub(crate) style: ButtonStyle,
    pub(crate) custom_id: CustomId,
    /// A standard Unicode emoji.
    pub(crate) emoji: Option<String>,
    pub(crate) label: Option<String>,
}

impl Button {
    /// The icon-only secondary gear accessory.
    #[requires(true)]
    #[ensures(ret.label.is_none() && ret.style == ButtonStyle::Secondary)]
    pub(crate) fn gear(custom_id: CustomId) -> Self {
        new!(Button {
            style: ButtonStyle::Secondary,
            custom_id,
            emoji: Some("⚙️".to_owned()),
            label: None,
        })
    }

    #[requires(true)]
    #[ensures(ret.get("type") == Some(&json!(TYPE_BUTTON)))]
    fn to_json(&self) -> Value {
        let mut value = json!({
            "type": TYPE_BUTTON,
            "style": self.style.code(),
            "custom_id": self.custom_id.as_str(),
        });
        if let Some(emoji) = &self.emoji {
            value["emoji"] = json!({ "name": emoji });
        }
        if let Some(label) = &self.label {
            value["label"] = json!(label);
        }
        value
    }
}

/// Text alongside an accessory. jbotci uses exactly one Section per message,
/// carrying the input block and the gear.
#[invariant(texts.len() <= MAX_SECTION_TEXTS)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section {
    pub(crate) texts: Vec1<TextDisplay>,
    pub(crate) accessory: Button,
}

#[invariant(description.as_ref().is_none_or(|description| utf16_len(description) <= MAX_ATTACHMENT_DESCRIPTION_UNITS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaItem {
    pub(crate) attachment: AttachmentName,
    pub(crate) description: Option<String>,
}

#[invariant(items.len() <= MAX_GALLERY_ITEMS)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaGallery {
    pub(crate) items: Vec1<MediaItem>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileComponent {
    pub(crate) attachment: AttachmentName,
}

#[invariant(::Section(_) => true)]
#[invariant(::TextDisplay(_) => true)]
#[invariant(::MediaGallery(_) => true)]
#[invariant(::File(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageComponent {
    Section(Section),
    TextDisplay(TextDisplay),
    MediaGallery(MediaGallery),
    File(FileComponent),
}

impl MessageComponent {
    /// Components this one contributes to the total count (itself plus its
    /// children and accessory, the way Discord counts).
    #[requires(true)]
    #[ensures(ret >= 1)]
    fn component_count(&self) -> usize {
        match self {
            Self::Section(section) => 1 + section.texts.len() + 1,
            Self::TextDisplay(_) | Self::File(_) => 1,
            Self::MediaGallery(_) => 1,
        }
    }

    /// Rendered text units this component contributes to the budget.
    #[requires(true)]
    #[ensures(true)]
    fn text_units(&self) -> usize {
        match self {
            Self::Section(section) => section.texts.iter().map(TextDisplay::units).sum(),
            Self::TextDisplay(text) => text.units(),
            Self::MediaGallery(_) | Self::File(_) => 0,
        }
    }

    /// Attachment names this component references.
    #[requires(true)]
    #[ensures(true)]
    fn attachment_references(&self) -> Vec<&AttachmentName> {
        match self {
            Self::MediaGallery(gallery) => {
                gallery.items.iter().map(|item| &item.attachment).collect()
            }
            Self::File(file) => vec![&file.attachment],
            Self::Section(_) | Self::TextDisplay(_) => Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn custom_ids(&self) -> Vec<&CustomId> {
        match self {
            Self::Section(section) => vec![&section.accessory.custom_id],
            Self::TextDisplay(_) | Self::MediaGallery(_) | Self::File(_) => Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_object())]
    fn to_json(&self) -> Value {
        match self {
            Self::Section(section) => json!({
                "type": TYPE_SECTION,
                "components": section.texts.iter().map(TextDisplay::to_json).collect::<Vec<_>>(),
                "accessory": section.accessory.to_json(),
            }),
            Self::TextDisplay(text) => text.to_json(),
            Self::MediaGallery(gallery) => json!({
                "type": TYPE_MEDIA_GALLERY,
                "items": gallery.items.iter().map(|item| {
                    let mut value = json!({ "media": { "url": item.attachment.reference() } });
                    if let Some(description) = &item.description {
                        value["description"] = json!(description);
                    }
                    value
                }).collect::<Vec<_>>(),
            }),
            Self::File(file) => json!({
                "type": TYPE_FILE,
                "file": { "url": file.attachment.reference() },
            }),
        }
    }
}

/// One attachment of the edited message: either kept from the current message
/// or uploaded with this request.
#[invariant(::Retain { .. } => true)]
#[invariant(::Upload { bytes, .. } => !bytes.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttachmentRequest {
    Retain {
        id: Snowflake,
        name: AttachmentName,
    },
    Upload {
        name: AttachmentName,
        content_type: &'static str,
        bytes: Vec<u8>,
    },
}

impl AttachmentRequest {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn name(&self) -> &AttachmentName {
        match self.as_data() {
            bityzba::data!(AttachmentRequest::Retain { name, .. })
            | bityzba::data!(AttachmentRequest::Upload { name, .. }) => name,
        }
    }
}

/// Why a message payload could not be assembled within bounds.
#[invariant(::TextBudget { .. } => true)]
#[invariant(::TooManyComponents { .. } => true)]
#[invariant(::UnreferencedOrMissingAttachment { .. } => true)]
#[invariant(::DuplicateCustomId { .. } => true)]
#[invariant(::DuplicateAttachmentName { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PayloadError {
    TextBudget { units: usize },
    TooManyComponents { top_level: usize, total: usize },
    UnreferencedOrMissingAttachment { name: String },
    DuplicateCustomId { custom_id: String },
    DuplicateAttachmentName { name: String },
}

impl fmt::Display for PayloadError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadError::TextBudget { units } => write!(
                formatter,
                "rendered text is {units} characters, over the {MESSAGE_TEXT_BUDGET_UNITS}-character message budget"
            ),
            PayloadError::TooManyComponents { top_level, total } => write!(
                formatter,
                "message has {top_level} top-level and {total} total components, over the limit"
            ),
            PayloadError::UnreferencedOrMissingAttachment { name } => write!(
                formatter,
                "attachment `{name}` is referenced by a component but not listed, or listed but shown by no component"
            ),
            PayloadError::DuplicateCustomId { custom_id } => {
                write!(formatter, "custom id `{custom_id}` appears twice")
            }
            PayloadError::DuplicateAttachmentName { name } => {
                write!(formatter, "attachment name `{name}` appears twice")
            }
        }
    }
}

impl std::error::Error for PayloadError {}

/// A complete Components V2 message body for create/edit.
///
/// The invariants pin the platform and application bounds: component counts,
/// the text budget, unique custom IDs and attachment names, and a one-to-one
/// correspondence between `attachment://` references and the attachment
/// list (every listed attachment is shown by a component, every reference
/// names a listed attachment), so no invisible or dangling attachment can be
/// published.
#[invariant(components.len() <= MAX_TOP_LEVEL_COMPONENTS)]
#[invariant(components.iter().map(MessageComponent::component_count).sum::<usize>() <= MAX_TOTAL_COMPONENTS)]
#[invariant(components.iter().map(MessageComponent::text_units).sum::<usize>() <= MESSAGE_TEXT_BUDGET_UNITS)]
#[invariant(
    attachments_match_references(components, attachments),
    "attachment references and the attachment list correspond one to one"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessagePayload {
    pub(crate) components: Vec1<MessageComponent>,
    pub(crate) attachments: Vec<AttachmentRequest>,
}

#[requires(true)]
#[ensures(true)]
fn attachments_match_references(
    components: &[MessageComponent],
    attachments: &[AttachmentRequest],
) -> bool {
    let mut referenced = components
        .iter()
        .flat_map(MessageComponent::attachment_references)
        .map(AttachmentName::as_str)
        .collect::<Vec<_>>();
    referenced.sort_unstable();
    let mut listed = attachments
        .iter()
        .map(|attachment| attachment.name().as_str())
        .collect::<Vec<_>>();
    listed.sort_unstable();
    referenced == listed && listed.windows(2).all(|pair| pair[0] != pair[1])
}

impl MessagePayload {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(
        components: Vec1<MessageComponent>,
        attachments: Vec<AttachmentRequest>,
    ) -> Result<Self, PayloadError> {
        let top_level = components.len();
        let total = components
            .iter()
            .map(MessageComponent::component_count)
            .sum::<usize>();
        if top_level > MAX_TOP_LEVEL_COMPONENTS || total > MAX_TOTAL_COMPONENTS {
            return Err(PayloadError::TooManyComponents { top_level, total });
        }
        let units = components
            .iter()
            .map(MessageComponent::text_units)
            .sum::<usize>();
        if units > MESSAGE_TEXT_BUDGET_UNITS {
            return Err(PayloadError::TextBudget { units });
        }
        let mut custom_ids: Vec<&str> = Vec::new();
        for custom_id in components.iter().flat_map(MessageComponent::custom_ids) {
            if custom_ids.contains(&custom_id.as_str()) {
                return Err(PayloadError::DuplicateCustomId {
                    custom_id: custom_id.as_str().to_owned(),
                });
            }
            custom_ids.push(custom_id.as_str());
        }
        let mut names: Vec<&str> = Vec::new();
        for attachment in &attachments {
            if names.contains(&attachment.name().as_str()) {
                return Err(PayloadError::DuplicateAttachmentName {
                    name: attachment.name().as_str().to_owned(),
                });
            }
            names.push(attachment.name().as_str());
        }
        if !attachments_match_references(&components, &attachments) {
            let referenced = components
                .iter()
                .flat_map(MessageComponent::attachment_references)
                .map(|name| name.as_str().to_owned())
                .collect::<Vec<_>>();
            let offending = referenced
                .iter()
                .find(|name| !names.contains(&name.as_str()))
                .cloned()
                .or_else(|| {
                    names
                        .iter()
                        .find(|name| !referenced.iter().any(|reference| reference == *name))
                        .map(|name| (*name).to_owned())
                })
                .unwrap_or_default();
            return Err(PayloadError::UnreferencedOrMissingAttachment { name: offending });
        }
        Ok(new!(MessagePayload {
            components,
            attachments,
        }))
    }

    /// Total rendered text units.
    #[requires(true)]
    #[ensures(ret <= MESSAGE_TEXT_BUDGET_UNITS)]
    pub(crate) fn text_units(&self) -> usize {
        self.components
            .iter()
            .map(MessageComponent::text_units)
            .sum()
    }

    /// The JSON body (`payload_json` when files are uploaded). Mentions are
    /// always disabled and `content`/`embeds` are reset as the V2 edit rules
    /// require.
    #[requires(true)]
    #[ensures(ret.get("flags") == Some(&json!(FLAG_IS_COMPONENTS_V2)))]
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "flags": FLAG_IS_COMPONENTS_V2,
            "content": Value::Null,
            "embeds": [],
            "allowed_mentions": { "parse": [] },
            "components": self.components.iter().map(MessageComponent::to_json).collect::<Vec<_>>(),
            "attachments": self.attachments.iter().enumerate().map(|(index, attachment)| match attachment.as_data() {
                bityzba::data!(AttachmentRequest::Retain { id, name }) => json!({ "id": id.as_str(), "filename": name.as_str() }),
                bityzba::data!(AttachmentRequest::Upload { name, .. }) => json!({ "id": index, "filename": name.as_str() }),
            }).collect::<Vec<_>>(),
        })
    }

    /// Uploads in `files[n]` order, `n` being the attachment's index in
    /// `attachments` (the placeholder id used in `to_json`).
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn uploads(&self) -> Vec<(usize, &AttachmentName, &'static str, &[u8])> {
        self.attachments
            .iter()
            .enumerate()
            .filter_map(|(index, attachment)| match attachment.as_data() {
                bityzba::data!(AttachmentRequest::Upload {
                    name,
                    content_type,
                    bytes,
                }) => Some((index, name, *content_type, bytes.as_slice())),
                bityzba::data!(AttachmentRequest::Retain { .. }) => None,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Modal components
// ---------------------------------------------------------------------------

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextInputStyle {
    Short,
    Paragraph,
}

#[invariant(value.as_ref().is_none_or(|value| utf16_len(value) <= MAX_TEXT_INPUT_UNITS))]
#[invariant(placeholder.as_ref().is_none_or(|placeholder| !placeholder.is_empty() && utf16_len(placeholder) <= MAX_PLACEHOLDER_UNITS))]
#[invariant(*max_length >= 1 && *max_length <= MAX_TEXT_INPUT_UNITS)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextInput {
    pub(crate) custom_id: CustomId,
    pub(crate) style: TextInputStyle,
    pub(crate) required: bool,
    pub(crate) value: Option<String>,
    pub(crate) placeholder: Option<String>,
    pub(crate) max_length: usize,
}

#[invariant(!label.is_empty() && utf16_len(label) <= MAX_OPTION_TEXT_UNITS)]
#[invariant(!value.is_empty() && utf16_len(value) <= MAX_OPTION_TEXT_UNITS)]
#[invariant(description.as_ref().is_none_or(|description| !description.is_empty() && utf16_len(description) <= MAX_OPTION_TEXT_UNITS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectOption {
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) description: Option<String>,
    pub(crate) default: bool,
}

#[requires(true)]
#[ensures(true)]
fn option_values_unique(options: &[SelectOption]) -> bool {
    options.iter().enumerate().all(|(index, option)| {
        !options[..index]
            .iter()
            .any(|seen| seen.value == option.value)
    })
}

#[invariant(options.len() <= MAX_SELECT_OPTIONS && option_values_unique(options))]
#[invariant(*min_values <= *max_values && *max_values <= options.len())]
#[invariant(!*required || *min_values >= 1, "a required select needs at least one choice")]
#[invariant(placeholder.as_ref().is_none_or(|placeholder| !placeholder.is_empty() && utf16_len(placeholder) <= MAX_SELECT_PLACEHOLDER_UNITS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StringSelect {
    pub(crate) custom_id: CustomId,
    pub(crate) options: Vec1<SelectOption>,
    pub(crate) min_values: usize,
    pub(crate) max_values: usize,
    pub(crate) required: bool,
    pub(crate) placeholder: Option<String>,
}

#[invariant(options.len() >= MIN_RADIO_OPTIONS && options.len() <= MAX_GROUP_OPTIONS && option_values_unique(options))]
#[invariant(options.iter().filter(|option| option.default).count() <= 1, "a radio group preselects at most one option")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RadioGroup {
    pub(crate) custom_id: CustomId,
    pub(crate) options: Vec1<SelectOption>,
    pub(crate) required: bool,
}

#[invariant(options.len() <= MAX_GROUP_OPTIONS && option_values_unique(options))]
#[invariant(*min_values <= *max_values && *max_values <= options.len() && *max_values >= 1)]
#[invariant(!*required || *min_values >= 1, "a required group needs at least one choice")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckboxGroup {
    pub(crate) custom_id: CustomId,
    pub(crate) options: Vec1<SelectOption>,
    pub(crate) min_values: usize,
    pub(crate) max_values: usize,
    pub(crate) required: bool,
}

#[invariant(::TextInput(_) => true)]
#[invariant(::StringSelect(_) => true)]
#[invariant(::RadioGroup(_) => true)]
#[invariant(::CheckboxGroup(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModalControl {
    TextInput(TextInput),
    StringSelect(StringSelect),
    RadioGroup(RadioGroup),
    CheckboxGroup(CheckboxGroup),
}

impl ModalControl {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn custom_id(&self) -> &CustomId {
        match self {
            Self::TextInput(control) => &control.custom_id,
            Self::StringSelect(control) => &control.custom_id,
            Self::RadioGroup(control) => &control.custom_id,
            Self::CheckboxGroup(control) => &control.custom_id,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_object())]
    fn to_json(&self) -> Value {
        match self {
            Self::TextInput(input) => {
                let mut value = json!({
                    "type": TYPE_TEXT_INPUT,
                    "custom_id": input.custom_id.as_str(),
                    "style": match input.style { TextInputStyle::Short => 1, TextInputStyle::Paragraph => 2 },
                    "required": input.required,
                    "max_length": input.max_length,
                });
                if let Some(text) = &input.value {
                    value["value"] = json!(text);
                }
                if let Some(placeholder) = &input.placeholder {
                    value["placeholder"] = json!(placeholder);
                }
                value
            }
            Self::StringSelect(select) => {
                let mut value = json!({
                    "type": TYPE_STRING_SELECT,
                    "custom_id": select.custom_id.as_str(),
                    "options": select.options.iter().map(select_option_json).collect::<Vec<_>>(),
                    "min_values": select.min_values,
                    "max_values": select.max_values,
                    "required": select.required,
                });
                if let Some(placeholder) = &select.placeholder {
                    value["placeholder"] = json!(placeholder);
                }
                value
            }
            Self::RadioGroup(group) => json!({
                "type": TYPE_RADIO_GROUP,
                "custom_id": group.custom_id.as_str(),
                "options": group.options.iter().map(select_option_json).collect::<Vec<_>>(),
                "required": group.required,
            }),
            Self::CheckboxGroup(group) => json!({
                "type": TYPE_CHECKBOX_GROUP,
                "custom_id": group.custom_id.as_str(),
                "options": group.options.iter().map(select_option_json).collect::<Vec<_>>(),
                "min_values": group.min_values,
                "max_values": group.max_values,
                "required": group.required,
            }),
        }
    }
}

#[requires(true)]
#[ensures(ret.is_object())]
fn select_option_json(option: &SelectOption) -> Value {
    let mut value = json!({ "label": option.label, "value": option.value });
    if let Some(description) = &option.description {
        value["description"] = json!(description);
    }
    if option.default {
        value["default"] = json!(true);
    }
    value
}

/// A Label wrapping one modal control.
#[invariant(!label.is_empty() && utf16_len(label) <= MAX_LABEL_UNITS)]
#[invariant(description.as_ref().is_none_or(|description| !description.is_empty() && utf16_len(description) <= MAX_LABEL_DESCRIPTION_UNITS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LabeledControl {
    pub(crate) label: String,
    pub(crate) description: Option<String>,
    pub(crate) control: ModalControl,
}

#[invariant(::TextDisplay(_) => true)]
#[invariant(::Label(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModalComponent {
    TextDisplay(TextDisplay),
    Label(LabeledControl),
}

impl ModalComponent {
    #[requires(true)]
    #[ensures(ret.is_object())]
    fn to_json(&self) -> Value {
        match self {
            Self::TextDisplay(text) => text.to_json(),
            Self::Label(labeled) => {
                let mut value = json!({
                    "type": TYPE_LABEL,
                    "label": labeled.label,
                    "component": labeled.control.to_json(),
                });
                if let Some(description) = &labeled.description {
                    value["description"] = json!(description);
                }
                value
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn modal_custom_ids_unique(components: &[ModalComponent]) -> bool {
    let ids = components
        .iter()
        .filter_map(|component| match component {
            ModalComponent::Label(labeled) => Some(labeled.control.custom_id().as_str()),
            ModalComponent::TextDisplay(_) => None,
        })
        .collect::<Vec<_>>();
    ids.iter()
        .enumerate()
        .all(|(index, id)| !ids[..index].contains(id))
}

/// A customization modal: at most five top-level components, unique control
/// IDs.
#[invariant(!title.is_empty() && utf16_len(title) <= MAX_MODAL_TITLE_UNITS)]
#[invariant(components.len() <= MAX_MODAL_COMPONENTS && modal_custom_ids_unique(components))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Modal {
    pub(crate) custom_id: CustomId,
    pub(crate) title: String,
    pub(crate) components: Vec1<ModalComponent>,
}

impl Modal {
    #[requires(true)]
    #[ensures(ret.get("custom_id").and_then(Value::as_str) == Some(self.custom_id.as_str()))]
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "custom_id": self.custom_id.as_str(),
            "title": self.title,
            "components": self.components.iter().map(ModalComponent::to_json).collect::<Vec<_>>(),
        })
    }
}

// ---------------------------------------------------------------------------
// Interaction responses
// ---------------------------------------------------------------------------

/// The immediate HTTP response to an interaction.
#[invariant(::EphemeralText { .. } => true)]
#[invariant(::Modal(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InteractionResponse {
    Pong,
    /// Public "thinking" placeholder that the follow-up edit replaces.
    DeferredChannelMessage,
    /// Component-based acknowledgement with no loading state; the message
    /// is edited afterwards through the same token.
    DeferredUpdateMessage,
    /// A private plain-text message to the acting user (errors, policy
    /// explanations). Mentions are disabled.
    EphemeralText {
        content: String,
    },
    Modal(Modal),
}

impl InteractionResponse {
    /// Build a private text response, bounding the text to Discord's content
    /// limit by ending it with an ellipsis rather than sending an invalid
    /// body. Error explanations are short by construction; this is a guard.
    #[requires(!content.trim().is_empty())]
    #[ensures(true)]
    pub(crate) fn ephemeral(content: &str) -> Self {
        let mut text = content.to_owned();
        if utf16_len(&text) > 2000 {
            let mut truncated = String::new();
            let mut units = 0;
            for character in text.chars() {
                let width = character.len_utf16();
                if units + width > 1999 {
                    break;
                }
                units += width;
                truncated.push(character);
            }
            truncated.push('…');
            text = truncated;
        }
        InteractionResponse::EphemeralText { content: text }
    }

    #[requires(true)]
    #[ensures(ret.get("type").is_some())]
    pub(crate) fn to_json(&self) -> Value {
        match self {
            InteractionResponse::Pong => json!({ "type": CALLBACK_PONG }),
            InteractionResponse::DeferredChannelMessage => {
                json!({ "type": CALLBACK_DEFERRED_CHANNEL_MESSAGE })
            }
            InteractionResponse::DeferredUpdateMessage => {
                json!({ "type": CALLBACK_DEFERRED_UPDATE_MESSAGE })
            }
            InteractionResponse::EphemeralText { content } => json!({
                "type": CALLBACK_CHANNEL_MESSAGE,
                "data": {
                    "content": content,
                    "flags": FLAG_EPHEMERAL,
                    "allowed_mentions": { "parse": [] },
                },
            }),
            InteractionResponse::Modal(modal) => json!({
                "type": CALLBACK_MODAL,
                "data": modal.to_json(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn text(content: &str) -> TextDisplay {
        TextDisplay::new(None, content.to_owned()).expect("text display")
    }

    #[requires(true)]
    #[ensures(true)]
    fn gear() -> Button {
        Button::gear(CustomId::new("j1.gi1.8.1.1.1.dev.AAAAAAAAAAA").expect("id"))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn message_payload_serializes_v2_shape_with_mentions_disabled() {
        let png = AttachmentName::new("gentufa.png").expect("name");
        let payload = MessagePayload::new(
            Vec1::try_from(vec![
                MessageComponent::Section(new!(Section {
                    texts: Vec1::new(
                        TextDisplay::new(Some(1001), "mi klama\n-# gentufa".to_owned())
                            .expect("input")
                    ),
                    accessory: gear(),
                })),
                MessageComponent::TextDisplay(text("(mi klama)")),
                MessageComponent::MediaGallery(new!(MediaGallery {
                    items: Vec1::new(new!(MediaItem {
                        attachment: png.clone(),
                        description: Some("gentufa diagram".to_owned()),
                    })),
                })),
            ])
            .expect("components"),
            vec![new!(AttachmentRequest::Upload {
                name: png,
                content_type: "image/png",
                bytes: vec![0x89, b'P', b'N', b'G'],
            })],
        )
        .expect("payload");
        let json = payload.to_json();
        assert_eq!(json["flags"], FLAG_IS_COMPONENTS_V2);
        assert_eq!(json["content"], Value::Null);
        assert_eq!(json["allowed_mentions"]["parse"], json!([]));
        assert_eq!(json["components"][0]["type"], 9);
        assert_eq!(json["components"][0]["components"][0]["id"], 1001);
        assert_eq!(json["components"][0]["accessory"]["style"], 2);
        assert_eq!(json["components"][0]["accessory"]["emoji"]["name"], "⚙️");
        assert!(json["components"][0]["accessory"].get("label").is_none());
        assert_eq!(
            json["components"][2]["items"][0]["media"]["url"],
            "attachment://gentufa.png"
        );
        assert_eq!(
            json["attachments"],
            json!([{ "id": 0, "filename": "gentufa.png" }])
        );
        assert_eq!(payload.uploads().len(), 1);
        assert_eq!(
            payload.text_units(),
            utf16_len("mi klama\n-# gentufa") + utf16_len("(mi klama)")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn payload_bounds_reject_budget_component_and_attachment_mismatches() {
        let big = "x".repeat(TEXT_DISPLAY_MAX_UNITS);
        let over_budget = MessagePayload::new(
            Vec1::try_from(vec![
                MessageComponent::TextDisplay(text(&big)),
                MessageComponent::TextDisplay(text("y")),
            ])
            .expect("components"),
            Vec::new(),
        );
        assert!(matches!(over_budget, Err(PayloadError::TextBudget { .. })));
        assert!(TextDisplay::new(None, format!("{big}x")).is_err());
        assert!(TextDisplay::new(None, String::new()).is_err());

        let too_many = MessagePayload::new(
            Vec1::try_from(
                (0..11)
                    .map(|_| MessageComponent::TextDisplay(text("a")))
                    .collect::<Vec<_>>(),
            )
            .expect("components"),
            Vec::new(),
        );
        assert!(matches!(
            too_many,
            Err(PayloadError::TooManyComponents { top_level: 11, .. })
        ));

        let dangling = MessagePayload::new(
            Vec1::new(MessageComponent::File(FileComponent {
                attachment: AttachmentName::new("jbotci-input.txt").expect("name"),
            })),
            Vec::new(),
        );
        assert!(matches!(
            dangling,
            Err(PayloadError::UnreferencedOrMissingAttachment { .. })
        ));

        let invisible = MessagePayload::new(
            Vec1::new(MessageComponent::TextDisplay(text("a"))),
            vec![new!(AttachmentRequest::Retain {
                id: Snowflake::parse("1").expect("snowflake"),
                name: AttachmentName::new("old.png").expect("name"),
            })],
        );
        assert!(matches!(
            invisible,
            Err(PayloadError::UnreferencedOrMissingAttachment { .. })
        ));
        assert!(AttachmentName::new("with space.png").is_err());
        assert!(AttachmentName::new("../etc").is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_serializes_labels_and_controls() {
        let modal = new!(Modal {
            custom_id: CustomId::new("j1m.g.1.1").expect("id"),
            title: "gentufa".to_owned(),
            components: Vec1::try_from(vec![
                ModalComponent::TextDisplay(text(
                    "[Open in app](https://jbotci.app/gentufa?text=mi)"
                )),
                ModalComponent::Label(new!(LabeledControl {
                    label: "Lojban text".to_owned(),
                    description: None,
                    control: ModalControl::TextInput(new!(TextInput {
                        custom_id: CustomId::new("text").expect("id"),
                        style: TextInputStyle::Paragraph,
                        required: true,
                        value: Some("mi klama".to_owned()),
                        placeholder: None,
                        max_length: MAX_TEXT_INPUT_UNITS,
                    })),
                })),
                ModalComponent::Label(new!(LabeledControl {
                    label: "Text view".to_owned(),
                    description: Some("How the parse is written".to_owned()),
                    control: ModalControl::RadioGroup(new!(RadioGroup {
                        custom_id: CustomId::new("view").expect("id"),
                        options: Vec1::try_from(vec![
                            new!(SelectOption {
                                label: "Brackets".to_owned(),
                                value: "brackets".to_owned(),
                                description: None,
                                default: true
                            }),
                            new!(SelectOption {
                                label: "Tree".to_owned(),
                                value: "tree".to_owned(),
                                description: None,
                                default: false
                            }),
                        ])
                        .expect("options"),
                        required: true,
                    })),
                })),
                ModalComponent::Label(new!(LabeledControl {
                    label: "Options".to_owned(),
                    description: None,
                    control: ModalControl::CheckboxGroup(new!(CheckboxGroup {
                        custom_id: CustomId::new("flags").expect("id"),
                        options: Vec1::new(new!(SelectOption {
                            label: "Include diagram".to_owned(),
                            value: "diagram".to_owned(),
                            description: None,
                            default: false
                        })),
                        min_values: 0,
                        max_values: 1,
                        required: false,
                    })),
                })),
                ModalComponent::Label(new!(LabeledControl {
                    label: "Page".to_owned(),
                    description: None,
                    control: ModalControl::StringSelect(new!(StringSelect {
                        custom_id: CustomId::new("page").expect("id"),
                        options: Vec1::new(new!(SelectOption {
                            label: "1".to_owned(),
                            value: "1".to_owned(),
                            description: None,
                            default: true
                        })),
                        min_values: 1,
                        max_values: 1,
                        required: true,
                        placeholder: None,
                    })),
                })),
            ])
            .expect("components"),
        });
        let json = InteractionResponse::Modal(modal).to_json();
        assert_eq!(json["type"], 9);
        let components = json["data"]["components"].as_array().expect("components");
        assert_eq!(components.len(), 5);
        assert_eq!(components[0]["type"], 10);
        assert_eq!(components[1]["type"], 18);
        assert_eq!(components[1]["component"]["type"], 4);
        assert_eq!(components[1]["component"]["style"], 2);
        assert_eq!(components[1]["component"]["value"], "mi klama");
        assert_eq!(components[2]["component"]["type"], 21);
        assert_eq!(components[2]["component"]["options"][0]["default"], true);
        assert_eq!(components[3]["component"]["type"], 22);
        assert_eq!(components[3]["component"]["min_values"], 0);
        assert_eq!(components[4]["component"]["type"], 3);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ephemeral_and_deferred_responses_serialize() {
        let ephemeral =
            InteractionResponse::ephemeral("Only the user who ran this command can change it.")
                .to_json();
        assert_eq!(ephemeral["type"], 4);
        assert_eq!(ephemeral["data"]["flags"], FLAG_EPHEMERAL);
        assert_eq!(ephemeral["data"]["allowed_mentions"]["parse"], json!([]));
        let long = InteractionResponse::ephemeral(&"a".repeat(2500));
        let InteractionResponse::EphemeralText { content } = &long else {
            panic!("ephemeral text");
        };
        assert!(utf16_len(content) <= 2000 && content.ends_with('…'));
        assert_eq!(InteractionResponse::Pong.to_json(), json!({ "type": 1 }));
        assert_eq!(
            InteractionResponse::DeferredChannelMessage.to_json(),
            json!({ "type": 5 })
        );
        assert_eq!(
            InteractionResponse::DeferredUpdateMessage.to_json(),
            json!({ "type": 6 })
        );
    }
}

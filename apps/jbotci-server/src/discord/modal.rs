//! The ⚙️ form: one modal per tool, and the request a submission means.
//!
//! Every form is built from the published request, so it opens on what the
//! message currently shows, and every control carries a typed value rather
//! than free text the parser would have to guess at. Discord allows five
//! top-level components, which is why related settings share one control: a
//! checkbox group for independent choices, a radio group for exclusive ones,
//! and, where a tool needs both a page and details, one select carrying both
//! whose cardinalities are checked on submission.
//!
//! A submission is read against the request it was opened from: settings the
//! form does not carry are kept, and a change to the source or to a filter
//! returns to the first page, because the old page number describes a result
//! set that no longer exists.

use std::collections::BTreeMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires, try_new};
use jbotci_gimfihi::{CollisionScope, GimfihiPreset, GismuShape, all_presets};
use serde_json::Value;
use vec1::Vec1;

use super::codec::ModalHeader;
use super::components::{
    BoundsError, CheckboxGroup, CustomId, LabeledControl, MAX_MODAL_COMPONENTS, Modal,
    ModalComponent, ModalControl, RadioGroup, SelectOption, StringSelect, TextDisplay, TextInput,
    TextInputStyle,
};
use super::links::APP_LINK_LABEL;
use super::request::{
    CuktaMode, CuktaOptions, CuktaRequest, CuktaResultKind, CuktaResultKindSet, DiscordRequest,
    DiscordTool, GentufaOptions, GentufaRequest, GentufaTextView, GimfihiOptions, GimfihiRequest,
    GismuShapeSet, JvozbaOptions, JvozbaRequest, JvozbaTarget, MAX_PAGE, MAX_SOURCE_UNITS,
    PageNumber, PublishedRequest, SourceText, VLACKU_MAX_PAGE, VlackuMode, VlackuOptions,
    VlackuRequest, VlackuWordType, VlackuWordTypeSet, VlaseiOptions, VlaseiRequest, VlaseiView,
    VlataiOptions, VlataiRequest,
};

/// Control identifiers. They are stable across builds because a modal opened
/// before a restart is submitted after it.
const ID_TEXT: &str = "text";
const ID_DIALECT: &str = "dialect";
const ID_VIEW: &str = "view";
const ID_FLAGS: &str = "flags";
const ID_QUERY: &str = "query";
const ID_MODE: &str = "mode";
const ID_WORD_TYPES: &str = "wordtypes";
const ID_PAGE_DETAILS: &str = "pagedetails";
const ID_KINDS: &str = "kinds";
const ID_PAGE: &str = "page";
const ID_PARTS: &str = "parts";
const ID_RAFSI: &str = "rafsi";
const ID_SOURCES: &str = "sources";
const ID_PRESET: &str = "preset";
const ID_OPTIONS: &str = "options";

/// Value prefixes inside the grouped selectors.
const PAGE_PREFIX: &str = "p";
const DETAIL_DECOMPOSE: &str = "d-decompose";
const DETAIL_ETYMOLOGY: &str = "d-etymology";
const SHAPE_PREFIX: &str = "shape-";
const COLLISION_PREFIX: &str = "collision-";
const FLAG_SHOW_COLLISIONS: &str = "show-collisions";
const FLAG_ALL_LETTERS: &str = "all-letters";
const FLAG_FREE_RAFSI: &str = "free-rafsi";
const FLAG_DIAGRAM: &str = "diagram";
const FLAG_ELIDED: &str = "elided";
const FLAG_COMPOUNDS: &str = "compounds";
const FLAG_GLOSSES: &str = "glosses";
const FLAG_STRESS: &str = "stress";
const FLAG_GLIDES: &str = "glides";
const FLAG_DETAILS: &str = "details";
const FLAG_DECOMPOSE: &str = "decompose";

/// Why a submission could not be read as a request.
#[invariant(::MissingControl { .. } => true)]
#[invariant(::UnknownValue { .. } => true)]
#[invariant(::WrongValueType { .. } => true)]
#[invariant(::DuplicateControl { .. } => true)]
#[invariant(::NonTextValue { .. } => true)]
#[invariant(::TooManyValues { .. } => true)]
#[invariant(::PageCount { .. } => true)]
#[invariant(::ShapeCount => true)]
#[invariant(::CollisionCount { .. } => true)]
#[invariant(::EmptyField { .. } => true)]
#[invariant(::Oversize { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubmissionError {
    /// A control the form always carries did not come back.
    MissingControl {
        control: &'static str,
    },
    UnknownValue {
        control: &'static str,
        value: String,
    },
    /// A control answered with a shape its kind never sends.
    WrongValueType {
        control: &'static str,
    },
    /// The same control answered twice.
    DuplicateControl {
        control: String,
    },
    /// A selection carried something that was not a string.
    NonTextValue {
        control: String,
    },
    /// A control that holds one value answered with several.
    TooManyValues {
        control: &'static str,
        selected: usize,
    },
    /// The combined selector must name exactly one page.
    PageCount {
        selected: usize,
    },
    /// At least one candidate shape must stay selected.
    ShapeCount,
    /// Exactly one collision scope must stay selected.
    CollisionCount {
        selected: usize,
    },
    EmptyField {
        control: &'static str,
    },
    Oversize {
        control: &'static str,
        units: usize,
    },
}

impl fmt::Display for SubmissionError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingControl { control } => {
                write!(formatter, "the form did not send its {control} field")
            }
            Self::UnknownValue { control, value } => write!(
                formatter,
                "the {control} field came back with a value this build does not know (`{value}`); reopen the form"
            ),
            Self::WrongValueType { control } => write!(
                formatter,
                "the {control} field came back in a shape its control does not use; reopen the form"
            ),
            Self::DuplicateControl { control } => {
                write!(formatter, "the form sent `{control}` twice")
            }
            Self::NonTextValue { control } => write!(
                formatter,
                "the {control} field carried a value that is not text"
            ),
            Self::TooManyValues { control, selected } => {
                write!(formatter, "choose one {control}; {selected} were selected")
            }
            Self::PageCount { selected } => write!(
                formatter,
                "choose exactly one page; {selected} were selected"
            ),
            Self::ShapeCount => write!(formatter, "choose at least one candidate shape"),
            Self::CollisionCount { selected } => write!(
                formatter,
                "choose exactly one collision setting; {selected} were selected"
            ),
            Self::EmptyField { control } => write!(formatter, "{control} cannot be empty"),
            Self::Oversize { control, units } => write!(
                formatter,
                "{control} is {units} characters, above the {MAX_SOURCE_UNITS}-character limit"
            ),
        }
    }
}

impl std::error::Error for SubmissionError {}

/// One control's value as Discord sent it back.
#[invariant(::Text(_) => true)]
#[invariant(::Selected(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum SubmittedValue {
    Text(String),
    Selected(Vec<String>),
}

/// The values of one modal submission, by control.
#[invariant(true)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Submission {
    values: BTreeMap<String, SubmittedValue>,
}

impl Submission {
    /// Read the values of a modal submission's `data`, following the shape a
    /// modal actually sends: `components` holds one Label per control, and
    /// the control inside answers with `value` (text and radio groups) or
    /// `values` (selects and checkbox groups). Anything else is refused
    /// rather than guessed at, so a malformed or stale payload cannot quietly
    /// change a setting.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn read(data: &Value) -> Result<Self, SubmissionError> {
        let mut values = BTreeMap::new();
        let components = data
            .get("components")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for component in components {
            // A Label carries its control; a control may also stand alone.
            let control = component.get("component").unwrap_or(component);
            let Some(custom_id) = control.get("custom_id").and_then(Value::as_str) else {
                continue;
            };
            let value = match (control.get("value"), control.get("values")) {
                (Some(Value::String(text)), None) => SubmittedValue::Text(text.clone()),
                // A radio group with nothing chosen answers with null.
                (Some(Value::Null), None) => SubmittedValue::Selected(Vec::new()),
                (None, Some(Value::Array(items))) => {
                    let mut selected = Vec::with_capacity(items.len());
                    for item in items {
                        let Some(text) = item.as_str() else {
                            return Err(SubmissionError::NonTextValue {
                                control: custom_id.to_owned(),
                            });
                        };
                        selected.push(text.to_owned());
                    }
                    SubmittedValue::Selected(selected)
                }
                _ => {
                    return Err(SubmissionError::NonTextValue {
                        control: custom_id.to_owned(),
                    });
                }
            };
            if values.insert(custom_id.to_owned(), value).is_some() {
                return Err(SubmissionError::DuplicateControl {
                    control: custom_id.to_owned(),
                });
            }
        }
        Ok(Submission { values })
    }

    /// A text control's value. A control that answered with a selection is a
    /// mismatch, not something to coerce.
    #[requires(true)]
    #[ensures(true)]
    fn text(&self, control: &'static str) -> Result<&str, SubmissionError> {
        match self.values.get(control) {
            Some(SubmittedValue::Text(text)) => Ok(text.as_str()),
            Some(SubmittedValue::Selected(_)) => Err(SubmissionError::WrongValueType { control }),
            None => Err(SubmissionError::MissingControl { control }),
        }
    }

    /// A text control's value when the form carries it. `None` means the form
    /// did not send the control at all, which is different from an empty one.
    #[requires(true)]
    #[ensures(true)]
    fn optional_text(&self, control: &'static str) -> Result<Option<&str>, SubmissionError> {
        match self.values.get(control) {
            Some(SubmittedValue::Text(text)) => Ok(Some(text.as_str())),
            Some(SubmittedValue::Selected(_)) => Err(SubmissionError::WrongValueType { control }),
            None => Ok(None),
        }
    }

    /// Every value a multi-valued control carries.
    #[requires(true)]
    #[ensures(true)]
    fn selected(&self, control: &'static str) -> Result<&[String], SubmissionError> {
        match self.values.get(control) {
            Some(SubmittedValue::Selected(values)) => Ok(values.as_slice()),
            Some(SubmittedValue::Text(_)) => Err(SubmissionError::WrongValueType { control }),
            None => Err(SubmissionError::MissingControl { control }),
        }
    }

    /// The single value of a control that holds one: a radio group answers
    /// with `value`, a one-choice select with a list of one.
    #[requires(true)]
    #[ensures(true)]
    fn chosen(&self, control: &'static str) -> Result<Option<&str>, SubmissionError> {
        match self.values.get(control) {
            Some(SubmittedValue::Text(text)) => Ok(Some(text.as_str())),
            Some(SubmittedValue::Selected(values)) => match values.as_slice() {
                [] => Ok(None),
                [value] => Ok(Some(value.as_str())),
                many => Err(SubmissionError::TooManyValues {
                    control,
                    selected: many.len(),
                }),
            },
            None => Err(SubmissionError::MissingControl { control }),
        }
    }
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

/// The form for `published`. `app_link` is the rendered link component for the
/// tools that have a page, already measured against its budget.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|modal| modal.components.len() <= MAX_MODAL_COMPONENTS) || ret.is_err())]
pub(crate) fn build(
    published: &PublishedRequest,
    app_link: Option<&str>,
) -> Result<Modal, BoundsError> {
    let tool = published.request.tool();
    let header = ModalHeader {
        tool,
        revision: published.revision,
        initiator: published.initiator.clone(),
    };
    let mut components = Vec::new();
    if let Some(link) = app_link {
        components.push(ModalComponent::TextDisplay(TextDisplay::new(
            None,
            link.to_owned(),
        )?));
    }
    components.extend(match &published.request {
        DiscordRequest::Gentufa(request) => gentufa_controls(request)?,
        DiscordRequest::Vlasei(request) => vlasei_controls(request)?,
        DiscordRequest::Vlatai(request) => vlatai_controls(request)?,
        DiscordRequest::Vlacku(request) => vlacku_controls(request)?,
        DiscordRequest::Cukta(request) => cukta_controls(request)?,
        DiscordRequest::Jvozba(request) => jvozba_controls(request)?,
        DiscordRequest::Gimfihi(request) => gimfihi_controls(request)?,
    });
    let components = Vec1::try_from_vec(components).expect("every tool has controls");
    let custom_id = CustomId::new(&header.encode())?;
    try_new!(Modal {
        custom_id,
        title: format!("Edit {tool}"),
        components,
    })
    .map_err(|_| BoundsError::new("modal", 1, MAX_MODAL_COMPONENTS))
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn labeled(
    label: &str,
    description: Option<&str>,
    control: ModalControl,
) -> Result<ModalComponent, BoundsError> {
    try_new!(LabeledControl {
        label: label.to_owned(),
        description: description.map(str::to_owned),
        control,
    })
    .map(ModalComponent::Label)
    .map_err(|_| BoundsError::new("modal label", label.chars().count(), 45))
}

#[requires(true)]
#[ensures(true)]
fn source_input(
    control: &str,
    value: Option<&str>,
    required: bool,
    placeholder: Option<&str>,
    multiline: bool,
) -> Result<ModalControl, BoundsError> {
    let custom_id = CustomId::new(control)?;
    try_new!(TextInput {
        custom_id,
        style: if multiline {
            TextInputStyle::Paragraph
        } else {
            TextInputStyle::Short
        },
        required,
        value: value.map(str::to_owned),
        placeholder: placeholder.map(str::to_owned),
        max_length: MAX_SOURCE_UNITS,
    })
    .map(ModalControl::TextInput)
    .map_err(|_| BoundsError::new("text input", value.map_or(0, str::len), MAX_SOURCE_UNITS))
}

#[requires(!value.is_empty() && !label.is_empty())]
#[ensures(ret.value == value)]
fn option(value: &str, label: &str, description: Option<&str>, default: bool) -> SelectOption {
    new!(SelectOption {
        label: label.to_owned(),
        value: value.to_owned(),
        description: description.map(str::to_owned),
        default,
    })
}

#[requires(options.len() >= 2)]
#[ensures(true)]
fn radio(control: &str, options: Vec<SelectOption>) -> Result<ModalControl, BoundsError> {
    let options =
        Vec1::try_from_vec(options).map_err(|_| BoundsError::new("radio group", 0, 10))?;
    let custom_id = CustomId::new(control)?;
    try_new!(RadioGroup {
        custom_id,
        options,
        required: true,
    })
    .map(ModalControl::RadioGroup)
    .map_err(|_| BoundsError::new("radio group", 0, 10))
}

#[requires(!options.is_empty())]
#[ensures(true)]
fn checkboxes(
    control: &str,
    options: Vec<SelectOption>,
    min_values: usize,
) -> Result<ModalControl, BoundsError> {
    let count = options.len();
    let options =
        Vec1::try_from_vec(options).map_err(|_| BoundsError::new("checkbox group", 0, 10))?;
    let custom_id = CustomId::new(control)?;
    try_new!(CheckboxGroup {
        custom_id,
        options,
        min_values,
        max_values: count,
        required: min_values > 0,
    })
    .map(ModalControl::CheckboxGroup)
    .map_err(|_| BoundsError::new("checkbox group", count, 10))
}

#[requires(!options.is_empty())]
#[ensures(true)]
fn select(
    control: &str,
    options: Vec<SelectOption>,
    min_values: usize,
    max_values: usize,
    placeholder: Option<&str>,
) -> Result<ModalControl, BoundsError> {
    let count = options.len();
    let options = Vec1::try_from_vec(options).map_err(|_| BoundsError::new("select", 0, 25))?;
    let custom_id = CustomId::new(control)?;
    try_new!(StringSelect {
        custom_id,
        options,
        min_values,
        max_values: max_values.min(count),
        required: min_values > 0,
        placeholder: placeholder.map(str::to_owned),
    })
    .map(ModalControl::StringSelect)
    .map_err(|_| BoundsError::new("select", count, 25))
}

/// Page options `1..=max`, with `current` preselected.
#[requires(max >= 2)]
#[ensures(ret.len() == max as usize)]
fn page_options(max: u8, current: PageNumber, prefix: &str) -> Vec<SelectOption> {
    (1..=max)
        .map(|page| {
            option(
                &format!("{prefix}{page}"),
                &format!("Page {page}"),
                None,
                page == current.get(),
            )
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn dialect_control(dialect: Option<&SourceText>) -> Result<ModalComponent, BoundsError> {
    labeled(
        "Dialect",
        Some("A dialect formula such as (cbm); leave empty for standard Lojban."),
        source_input(
            ID_DIALECT,
            Some(dialect.map(SourceText::as_str).unwrap_or_default()),
            false,
            Some("(cbm)"),
            false,
        )?,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 4) || ret.is_err())]
fn gentufa_controls(request: &GentufaRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    let options = request.options;
    Ok(vec![
        labeled(
            "Text",
            Some("The Lojban text to parse."),
            source_input(ID_TEXT, Some(request.text.as_str()), true, None, true)?,
        )?,
        dialect_control(request.dialect.as_ref())?,
        labeled(
            "Text view",
            None,
            radio(
                ID_VIEW,
                vec![
                    option(
                        "brackets",
                        "Brackets",
                        Some("The parse as nested brackets."),
                        options.view == GentufaTextView::Brackets,
                    ),
                    option(
                        "tree",
                        "Tree",
                        Some("The parse as an indented tree."),
                        options.view == GentufaTextView::Tree,
                    ),
                ],
            )?,
        )?,
        labeled(
            "Show",
            None,
            checkboxes(
                ID_FLAGS,
                vec![
                    option(
                        FLAG_DIAGRAM,
                        "Diagram image",
                        Some("Attach the block diagram as a PNG."),
                        options.include_diagram,
                    ),
                    option(FLAG_ELIDED, "Elided terminators", None, options.show_elided),
                    option(
                        FLAG_COMPOUNDS,
                        "Attested compounds",
                        Some("Keep dictionary compounds together."),
                        options.show_compounds,
                    ),
                    option(FLAG_GLOSSES, "Glosses", None, options.show_glosses),
                ],
                0,
            )?,
        )?,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 4) || ret.is_err())]
fn vlasei_controls(request: &VlaseiRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    let options = request.options;
    Ok(vec![
        labeled(
            "Text",
            Some("The Lojban text to segment."),
            source_input(ID_TEXT, Some(request.text.as_str()), true, None, true)?,
        )?,
        dialect_control(request.dialect.as_ref())?,
        labeled(
            "View",
            None,
            radio(
                ID_VIEW,
                vec![
                    option("words", "Words", None, options.view == VlaseiView::Words),
                    option(
                        "brackets",
                        "Brackets",
                        None,
                        options.view == VlaseiView::Brackets,
                    ),
                    option("tree", "Tree", None, options.view == VlaseiView::Tree),
                    option("ipa", "IPA", None, options.view == VlaseiView::Ipa),
                ],
            )?,
        )?,
        labeled(
            "Detail",
            None,
            checkboxes(
                ID_FLAGS,
                vec![option(
                    FLAG_DECOMPOSE,
                    "Decompose lujvo",
                    Some("Show the rafsi a lujvo is built from."),
                    options.decompose_lujvo,
                )],
                0,
            )?,
        )?,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 3) || ret.is_err())]
fn vlatai_controls(request: &VlataiRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    let options = request.options;
    Ok(vec![
        labeled(
            "Word",
            Some("One word to analyse."),
            source_input(ID_TEXT, Some(request.text.as_str()), true, None, false)?,
        )?,
        dialect_control(request.dialect.as_ref())?,
        labeled(
            "Show",
            None,
            checkboxes(
                ID_FLAGS,
                vec![
                    option(FLAG_STRESS, "Stress marks", None, options.mark_stress),
                    option(FLAG_GLIDES, "Glide marks", None, options.mark_glides),
                    option(
                        FLAG_DETAILS,
                        "Formation details",
                        Some("Possible rafsi, lujvo parts, fu'ivla stage."),
                        options.show_details,
                    ),
                ],
                0,
            )?,
        )?,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 4) || ret.is_err())]
fn vlacku_controls(request: &VlackuRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    let options = request.options;
    let mut page_and_details = page_options(VLACKU_MAX_PAGE, options.page, PAGE_PREFIX);
    page_and_details.push(option(
        DETAIL_DECOMPOSE,
        "Show lujvo decomposition",
        None,
        options.decompose_lujvo,
    ));
    page_and_details.push(option(
        DETAIL_ETYMOLOGY,
        "Show etymology",
        None,
        options.show_etymology,
    ));
    Ok(vec![
        labeled(
            "Query",
            None,
            source_input(ID_QUERY, Some(request.query.as_str()), true, None, false)?,
        )?,
        labeled(
            "Search",
            None,
            radio(
                ID_MODE,
                VlackuMode::ALL
                    .iter()
                    .map(|mode| {
                        option(
                            mode.slash_value(),
                            mode.label(),
                            None,
                            *mode == options.mode,
                        )
                    })
                    .collect(),
            )?,
        )?,
        labeled(
            "Word types",
            Some("Leave empty to search every kind of word."),
            checkboxes(
                ID_WORD_TYPES,
                VlackuWordType::ALL
                    .iter()
                    .map(|word_type| {
                        option(
                            word_type.filter_value(),
                            word_type.filter_value(),
                            None,
                            options.word_types.contains(*word_type),
                        )
                    })
                    .collect(),
                0,
            )?,
        )?,
        labeled(
            "Page and details",
            Some("Choose one page; the other choices are independent."),
            select(ID_PAGE_DETAILS, page_and_details, 1, 3, Some("Page 1"))?,
        )?,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 4) || ret.is_err())]
fn cukta_controls(request: &CuktaRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    let options = request.options;
    Ok(vec![
        labeled(
            "Query or reference",
            Some("A search query, or a reference such as 5.2; empty for the contents."),
            source_input(
                ID_QUERY,
                Some(
                    request
                        .query
                        .as_ref()
                        .map(SourceText::as_str)
                        .unwrap_or_default(),
                ),
                false,
                Some("tanru"),
                false,
            )?,
        )?,
        labeled(
            "Mode",
            None,
            radio(
                ID_MODE,
                CuktaMode::ALL
                    .iter()
                    .map(|mode| {
                        option(
                            mode.slash_value(),
                            mode.label(),
                            None,
                            *mode == options.mode,
                        )
                    })
                    .collect(),
            )?,
        )?,
        labeled(
            "Result kinds",
            Some("Leave empty to search every kind of passage."),
            checkboxes(
                ID_KINDS,
                CuktaResultKind::ALL
                    .iter()
                    .map(|kind| {
                        option(
                            kind.value(),
                            kind.label(),
                            None,
                            options.kinds.contains(*kind),
                        )
                    })
                    .collect(),
                0,
            )?,
        )?,
        labeled(
            "Page",
            None,
            select(
                ID_PAGE,
                page_options(MAX_PAGE, options.page, ""),
                1,
                1,
                Some("Page 1"),
            )?,
        )?,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 3) || ret.is_err())]
fn jvozba_controls(request: &JvozbaRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    Ok(vec![
        labeled(
            "Words",
            Some("The words to combine, separated by spaces."),
            source_input(ID_PARTS, Some(request.parts.as_str()), true, None, false)?,
        )?,
        labeled(
            "Fixed rafsi",
            Some("Rafsi to use as given, separated by spaces or commas."),
            source_input(
                ID_RAFSI,
                Some(
                    request
                        .rafsi
                        .as_ref()
                        .map(SourceText::as_str)
                        .unwrap_or_default(),
                ),
                false,
                None,
                false,
            )?,
        )?,
        labeled(
            "Build",
            None,
            radio(
                ID_MODE,
                vec![
                    option(
                        JvozbaTarget::Lujvo.slash_value(),
                        "Lujvo",
                        None,
                        request.options.target == JvozbaTarget::Lujvo,
                    ),
                    option(
                        JvozbaTarget::Cmevla.slash_value(),
                        "Cmevla",
                        None,
                        request.options.target == JvozbaTarget::Cmevla,
                    ),
                ],
            )?,
        )?,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|controls| controls.len() == 4) || ret.is_err())]
fn gimfihi_controls(request: &GimfihiRequest) -> Result<Vec<ModalComponent>, BoundsError> {
    let options = request.options;
    let mut candidate_options = vec![
        option(
            &format!("{SHAPE_PREFIX}{}", GismuShape::Ccvcv.as_str()),
            "Shape ccvcv",
            None,
            options.shapes.is_empty() || options.shapes.contains(GismuShape::Ccvcv),
        ),
        option(
            &format!("{SHAPE_PREFIX}{}", GismuShape::Cvccv.as_str()),
            "Shape cvccv",
            None,
            options.shapes.is_empty() || options.shapes.contains(GismuShape::Cvccv),
        ),
    ];
    for scope in [
        CollisionScope::All,
        CollisionScope::Official,
        CollisionScope::None,
    ] {
        candidate_options.push(option(
            &format!("{COLLISION_PREFIX}{}", scope.as_str()),
            match scope {
                CollisionScope::All => "Check all gismu",
                CollisionScope::Official => "Check official gismu",
                CollisionScope::None => "Do not check collisions",
            },
            None,
            options.collisions == scope,
        ));
    }
    candidate_options.push(option(
        FLAG_SHOW_COLLISIONS,
        "Show colliding candidates",
        None,
        options.show_collisions,
    ));
    candidate_options.push(option(
        FLAG_ALL_LETTERS,
        "Score all letters",
        None,
        options.all_letters,
    ));
    candidate_options.push(option(
        FLAG_FREE_RAFSI,
        "Require a free short rafsi",
        None,
        options.require_free_short_rafsi,
    ));
    Ok(vec![
        labeled(
            "Source words",
            Some("LANG[:WEIGHT]:WORD records, separated by commas."),
            source_input(
                ID_SOURCES,
                Some(
                    request
                        .sources
                        .as_ref()
                        .map(SourceText::as_str)
                        .unwrap_or_default(),
                ),
                false,
                Some("eng:5:go, spa:3:[ir]"),
                true,
            )?,
        )?,
        labeled(
            "Preset",
            Some("Language weights; leave empty to weight the sources yourself."),
            select(
                ID_PRESET,
                all_presets()
                    .iter()
                    .map(|preset| {
                        option(
                            preset.as_str(),
                            preset.as_str(),
                            None,
                            options.preset == Some(*preset),
                        )
                    })
                    .collect(),
                0,
                1,
                Some("No preset"),
            )?,
        )?,
        labeled(
            "Candidates",
            Some("At least one shape, and exactly one collision setting."),
            checkboxes(ID_OPTIONS, candidate_options, 0)?,
        )?,
        labeled(
            "Page",
            None,
            select(
                ID_PAGE,
                page_options(MAX_PAGE, options.page, ""),
                1,
                1,
                Some("Page 1"),
            )?,
        )?,
    ])
}

// ---------------------------------------------------------------------------
// Reading a submission
// ---------------------------------------------------------------------------

/// The request `submission` describes, read against the request its form was
/// opened from. A change to the source text or to a filter returns to the
/// first page: the page it named describes results that no longer exist.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|request| request.tool() == published.tool()) || ret.is_err())]
pub(crate) fn parse_submission(
    published: &DiscordRequest,
    submission: &Submission,
) -> Result<DiscordRequest, SubmissionError> {
    match published {
        DiscordRequest::Gentufa(previous) => {
            let options = flags(
                submission,
                ID_FLAGS,
                "show",
                &[FLAG_DIAGRAM, FLAG_ELIDED, FLAG_COMPOUNDS, FLAG_GLOSSES],
            )?;
            Ok(DiscordRequest::Gentufa(GentufaRequest {
                text: required_source(submission, ID_TEXT, "text")?,
                dialect: optional_source(
                    submission,
                    ID_DIALECT,
                    "dialect",
                    previous.dialect.as_ref(),
                )?,
                options: GentufaOptions {
                    view: match chosen_value(submission, ID_VIEW, "view")? {
                        "tree" => GentufaTextView::Tree,
                        "brackets" => GentufaTextView::Brackets,
                        value => return Err(unknown("view", value)),
                    },
                    include_diagram: has(&options, FLAG_DIAGRAM),
                    show_elided: has(&options, FLAG_ELIDED),
                    show_compounds: has(&options, FLAG_COMPOUNDS),
                    show_glosses: has(&options, FLAG_GLOSSES),
                },
            }))
        }
        DiscordRequest::Vlasei(previous) => {
            let flags = flags(submission, ID_FLAGS, "detail", &[FLAG_DECOMPOSE])?;
            Ok(DiscordRequest::Vlasei(VlaseiRequest {
                text: required_source(submission, ID_TEXT, "text")?,
                dialect: optional_source(
                    submission,
                    ID_DIALECT,
                    "dialect",
                    previous.dialect.as_ref(),
                )?,
                options: VlaseiOptions {
                    view: match chosen_value(submission, ID_VIEW, "view")? {
                        "words" => VlaseiView::Words,
                        "brackets" => VlaseiView::Brackets,
                        "tree" => VlaseiView::Tree,
                        "ipa" => VlaseiView::Ipa,
                        value => return Err(unknown("view", value)),
                    },
                    decompose_lujvo: has(&flags, FLAG_DECOMPOSE),
                },
            }))
        }
        DiscordRequest::Vlatai(previous) => {
            let flags = flags(
                submission,
                ID_FLAGS,
                "show",
                &[FLAG_STRESS, FLAG_GLIDES, FLAG_DETAILS],
            )?;
            Ok(DiscordRequest::Vlatai(VlataiRequest {
                text: required_source(submission, ID_TEXT, "word")?,
                dialect: optional_source(
                    submission,
                    ID_DIALECT,
                    "dialect",
                    previous.dialect.as_ref(),
                )?,
                options: VlataiOptions {
                    mark_stress: has(&flags, FLAG_STRESS),
                    mark_glides: has(&flags, FLAG_GLIDES),
                    show_details: has(&flags, FLAG_DETAILS),
                },
            }))
        }
        DiscordRequest::Vlacku(previous) => {
            let query = required_source(submission, ID_QUERY, "query")?;
            let mode = VlackuMode::from_slash_value(chosen_value(submission, ID_MODE, "search")?)
                .ok_or_else(|| {
                unknown(
                    "search",
                    chosen_value(submission, ID_MODE, "search").unwrap_or(""),
                )
            })?;
            let mut word_types = VlackuWordTypeSet::empty();
            for value in submission.selected(ID_WORD_TYPES)? {
                let word_type = VlackuWordType::ALL
                    .iter()
                    .find(|word_type| word_type.filter_value() == value)
                    .ok_or_else(|| unknown("word types", value))?;
                word_types = word_types.with(*word_type);
            }
            let selected = submission.selected(ID_PAGE_DETAILS)?;
            let mut pages = Vec::new();
            let mut seen = Vec::new();
            for value in selected {
                if seen.contains(&value.as_str()) {
                    return Err(unknown("page and details", value));
                }
                seen.push(value.as_str());
                if let Some(page) = value.strip_prefix(PAGE_PREFIX)
                    && !page.is_empty()
                    && page.chars().all(|character| character.is_ascii_digit())
                {
                    pages.push(page);
                } else if value != DETAIL_DECOMPOSE && value != DETAIL_ETYMOLOGY {
                    return Err(unknown("page and details", value));
                }
            }
            let [page] = pages.as_slice() else {
                return Err(SubmissionError::PageCount {
                    selected: pages.len(),
                });
            };
            let page = page
                .parse::<u8>()
                .ok()
                .and_then(PageNumber::new)
                .filter(|page| page.get() <= VLACKU_MAX_PAGE)
                .ok_or_else(|| unknown("page", page))?;
            let filters_changed = query != previous.query
                || mode != previous.options.mode
                || word_types != previous.options.word_types;
            Ok(DiscordRequest::Vlacku(new!(VlackuRequest {
                query,
                options: VlackuOptions {
                    mode,
                    word_types,
                    decompose_lujvo: selected.iter().any(|value| value == DETAIL_DECOMPOSE),
                    show_etymology: selected.iter().any(|value| value == DETAIL_ETYMOLOGY),
                    page: if filters_changed {
                        PageNumber::first()
                    } else {
                        page
                    },
                },
            })))
        }
        DiscordRequest::Cukta(previous) => {
            let query = optional_source(submission, ID_QUERY, "query", previous.query.as_ref())?;
            let mode = CuktaMode::from_slash_value(chosen_value(submission, ID_MODE, "mode")?)
                .ok_or_else(|| {
                    unknown(
                        "mode",
                        chosen_value(submission, ID_MODE, "mode").unwrap_or(""),
                    )
                })?;
            if mode.requires_query()
                && query
                    .as_ref()
                    .is_none_or(|query| query.as_str().trim().is_empty())
            {
                return Err(SubmissionError::EmptyField {
                    control: "query or reference",
                });
            }
            let mut kinds = CuktaResultKindSet::empty();
            for value in submission.selected(ID_KINDS)? {
                let kind = CuktaResultKind::ALL
                    .iter()
                    .find(|kind| kind.value() == value)
                    .ok_or_else(|| unknown("result kinds", value))?;
                kinds = kinds.with(*kind);
            }
            let page = single_page(submission, ID_PAGE, MAX_PAGE)?;
            let filters_changed = query.as_ref().map(SourceText::as_str)
                != previous.query.as_ref().map(SourceText::as_str)
                || mode != previous.options.mode
                || kinds != previous.options.kinds;
            Ok(DiscordRequest::Cukta(CuktaRequest {
                query,
                options: CuktaOptions {
                    mode,
                    kinds,
                    page: if filters_changed {
                        PageNumber::first()
                    } else {
                        page
                    },
                },
            }))
        }
        DiscordRequest::Jvozba(previous) => Ok(DiscordRequest::Jvozba(JvozbaRequest {
            parts: required_source(submission, ID_PARTS, "words")?,
            rafsi: optional_source(submission, ID_RAFSI, "fixed rafsi", previous.rafsi.as_ref())?,
            options: JvozbaOptions {
                target: JvozbaTarget::from_slash_value(chosen_value(submission, ID_MODE, "build")?)
                    .ok_or_else(|| {
                        unknown(
                            "build",
                            chosen_value(submission, ID_MODE, "build").unwrap_or(""),
                        )
                    })?,
            },
        })),
        DiscordRequest::Gimfihi(previous) => {
            let sources = optional_source(
                submission,
                ID_SOURCES,
                "source words",
                previous.sources.as_ref(),
            )?;
            let selected = submission.selected(ID_OPTIONS)?;
            let mut shapes = GismuShapeSet::empty();
            let mut scopes = Vec::new();
            for value in selected {
                if let Some(shape) = value.strip_prefix(SHAPE_PREFIX) {
                    let shape = [GismuShape::Ccvcv, GismuShape::Cvccv]
                        .into_iter()
                        .find(|candidate| candidate.as_str() == shape)
                        .ok_or_else(|| unknown("candidates", value))?;
                    shapes = shapes.with(shape);
                } else if let Some(scope) = value.strip_prefix(COLLISION_PREFIX) {
                    scopes.push(
                        [
                            CollisionScope::All,
                            CollisionScope::Official,
                            CollisionScope::None,
                        ]
                        .into_iter()
                        .find(|candidate| candidate.as_str() == scope)
                        .ok_or_else(|| unknown("candidates", value))?,
                    );
                } else if !matches!(
                    value.as_str(),
                    FLAG_SHOW_COLLISIONS | FLAG_ALL_LETTERS | FLAG_FREE_RAFSI
                ) {
                    return Err(unknown("candidates", value));
                }
            }
            if shapes.is_empty() {
                return Err(SubmissionError::ShapeCount);
            }
            let [collisions] = scopes.as_slice() else {
                return Err(SubmissionError::CollisionCount {
                    selected: scopes.len(),
                });
            };
            let preset = match submission.chosen(ID_PRESET)? {
                Some(value) if !value.is_empty() => Some(
                    all_presets()
                        .iter()
                        .find(|preset| preset.as_str() == value)
                        .copied()
                        .ok_or_else(|| unknown("preset", value))?,
                ),
                _ => None,
            };
            let page = single_page(submission, ID_PAGE, MAX_PAGE)?;
            let all_letters = has(&selected.to_vec(), FLAG_ALL_LETTERS);
            let require_free_short_rafsi = has(&selected.to_vec(), FLAG_FREE_RAFSI);
            // Everything that changes which candidates exist starts the list
            // over; showing collisions only changes how it reads.
            let generation_changed = sources.as_ref().map(SourceText::as_str)
                != previous.sources.as_ref().map(SourceText::as_str)
                || preset != previous.options.preset
                || shapes != previous.options.shapes
                || *collisions != previous.options.collisions
                || all_letters != previous.options.all_letters
                || require_free_short_rafsi != previous.options.require_free_short_rafsi;
            Ok(DiscordRequest::Gimfihi(GimfihiRequest {
                sources,
                options: GimfihiOptions {
                    preset,
                    shapes,
                    collisions: *collisions,
                    show_collisions: selected.iter().any(|value| value == FLAG_SHOW_COLLISIONS),
                    all_letters,
                    require_free_short_rafsi,
                    page: if generation_changed {
                        PageNumber::first()
                    } else {
                        page
                    },
                },
            }))
        }
    }
}

#[requires(!value.is_empty() || true)]
#[ensures(true)]
fn unknown(control: &'static str, value: &str) -> SubmissionError {
    SubmissionError::UnknownValue {
        control,
        value: value.to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
fn chosen_value<'a>(
    submission: &'a Submission,
    control: &'static str,
    name: &'static str,
) -> Result<&'a str, SubmissionError> {
    submission
        .chosen(control)?
        .ok_or(SubmissionError::MissingControl { control: name })
}

#[requires(true)]
#[ensures(true)]
fn single_page(
    submission: &Submission,
    control: &'static str,
    max: u8,
) -> Result<PageNumber, SubmissionError> {
    let selected = submission.selected(control)?;
    let [page] = selected else {
        return Err(SubmissionError::PageCount {
            selected: selected.len(),
        });
    };
    page.parse::<u8>()
        .ok()
        .filter(|page| *page <= max)
        .and_then(PageNumber::new)
        .ok_or_else(|| unknown("page", page))
}

/// The selected values of a checkbox group, each of which must be one this
/// build offers: a value from another build is a stale form, not a setting to
/// apply, and a value repeated twice is a payload to refuse.
#[requires(!allowed.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|selected| selected.len() <= allowed.len()) || ret.is_err())]
fn flags(
    submission: &Submission,
    control: &'static str,
    name: &'static str,
    allowed: &[&str],
) -> Result<Vec<String>, SubmissionError> {
    let selected = submission.selected(control)?;
    let mut known = Vec::with_capacity(selected.len());
    for value in selected {
        if !allowed.contains(&value.as_str()) || known.contains(value) {
            return Err(unknown(name, value));
        }
        known.push(value.clone());
    }
    Ok(known)
}

/// Whether a checkbox group's selection carries `name`.
#[requires(!name.is_empty())]
#[ensures(ret == selected.iter().any(|value| value == name))]
fn has(selected: &[String], name: &str) -> bool {
    selected.iter().any(|value| value == name)
}

#[requires(true)]
#[ensures(true)]
fn required_source(
    submission: &Submission,
    control: &'static str,
    name: &'static str,
) -> Result<SourceText, SubmissionError> {
    let text = submission.text(control)?;
    if text.trim().is_empty() {
        return Err(SubmissionError::EmptyField { control: name });
    }
    SourceText::new(text).map_err(|_| SubmissionError::Oversize {
        control: name,
        units: super::request::utf16_len(text),
    })
}

/// An optional field's value, read against what the request already held.
/// An empty control means "still nothing" when the field was absent and "the
/// reader cleared it" when it held something, so a form submitted unchanged
/// returns the request it was opened from. Text is taken verbatim: spacing is
/// the reader's, and validation refuses rather than rewrites.
#[requires(true)]
#[ensures(true)]
fn optional_source(
    submission: &Submission,
    control: &'static str,
    name: &'static str,
    previous: Option<&SourceText>,
) -> Result<Option<SourceText>, SubmissionError> {
    let Some(text) = submission.optional_text(control)? else {
        return Ok(None);
    };
    if text.is_empty() && previous.is_none() {
        return Ok(None);
    }
    SourceText::new(text)
        .map(Some)
        .map_err(|_| SubmissionError::Oversize {
            control: name,
            units: super::request::utf16_len(text),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::components::{MAX_MODAL_TITLE_UNITS, MAX_SELECT_OPTIONS};
    use crate::discord::request::{BuildTag, Revision, Snowflake, utf16_len};

    #[requires(true)]
    #[ensures(true)]
    fn source(text: &str) -> SourceText {
        SourceText::new(text).expect("text")
    }

    #[requires(true)]
    #[ensures(true)]
    fn published(request: DiscordRequest) -> PublishedRequest {
        PublishedRequest {
            request,
            revision: Revision::INITIAL,
            initiator: Snowflake::parse("123456789012345678").expect("snowflake"),
            build_tag: BuildTag::current(),
        }
    }

    /// Every tool's request, with settings away from their defaults so a
    /// round trip has something to lose.
    #[requires(true)]
    #[ensures(ret.len() == 7)]
    fn every_request() -> Vec<DiscordRequest> {
        vec![
            DiscordRequest::Gentufa(GentufaRequest {
                text: source("mi klama lo zarci"),
                dialect: Some(source("(cbm)")),
                options: GentufaOptions {
                    view: GentufaTextView::Tree,
                    include_diagram: true,
                    show_elided: true,
                    show_compounds: false,
                    show_glosses: true,
                },
            }),
            DiscordRequest::Vlasei(VlaseiRequest {
                text: source("klabajra"),
                dialect: None,
                options: VlaseiOptions {
                    view: VlaseiView::Ipa,
                    decompose_lujvo: true,
                },
            }),
            DiscordRequest::Vlatai(VlataiRequest {
                text: source("klama"),
                dialect: Some(source("")),
                options: VlataiOptions {
                    mark_stress: false,
                    mark_glides: true,
                    show_details: false,
                },
            }),
            DiscordRequest::Vlacku(new!(VlackuRequest {
                query: source("klama"),
                options: VlackuOptions {
                    mode: VlackuMode::Rafsi,
                    word_types: VlackuWordTypeSet::empty().with(VlackuWordType::Gismu),
                    decompose_lujvo: false,
                    show_etymology: true,
                    page: PageNumber::new(4).expect("page"),
                },
            })),
            DiscordRequest::Cukta(CuktaRequest {
                query: Some(source("tanru")),
                options: CuktaOptions {
                    mode: CuktaMode::Word,
                    kinds: CuktaResultKindSet::empty().with(CuktaResultKind::Example),
                    page: PageNumber::new(7).expect("page"),
                },
            }),
            DiscordRequest::Jvozba(JvozbaRequest {
                parts: source("klama bajra"),
                rafsi: Some(source("kla")),
                options: JvozbaOptions {
                    target: JvozbaTarget::Cmevla,
                },
            }),
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: Some(source("eng:5:go, spa:3:[ir]")),
                options: GimfihiOptions {
                    preset: Some(GimfihiPreset::Ilmen6),
                    shapes: GismuShapeSet::empty().with(GismuShape::Cvccv),
                    collisions: CollisionScope::Official,
                    show_collisions: true,
                    all_letters: false,
                    require_free_short_rafsi: true,
                    page: PageNumber::new(3).expect("page"),
                },
            }),
        ]
    }

    /// Every tool's request at its defaults, with optional fields absent.
    #[requires(true)]
    #[ensures(ret.len() == 7)]
    fn default_requests() -> Vec<DiscordRequest> {
        vec![
            DiscordRequest::Gentufa(GentufaRequest {
                text: source("mi klama"),
                dialect: None,
                options: GentufaOptions::default(),
            }),
            DiscordRequest::Vlasei(VlaseiRequest {
                text: source("mi klama"),
                dialect: None,
                options: VlaseiOptions::default(),
            }),
            DiscordRequest::Vlatai(VlataiRequest {
                text: source("klama"),
                dialect: None,
                options: VlataiOptions::default(),
            }),
            DiscordRequest::Vlacku(new!(VlackuRequest {
                query: source("klama"),
                options: VlackuOptions::default(),
            })),
            DiscordRequest::Cukta(CuktaRequest {
                query: None,
                options: CuktaOptions {
                    mode: CuktaMode::Contents,
                    kinds: CuktaResultKindSet::empty(),
                    page: PageNumber::first(),
                },
            }),
            DiscordRequest::Jvozba(JvozbaRequest {
                parts: source("klama bajra"),
                rafsi: None,
                options: JvozbaOptions::default(),
            }),
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: None,
                options: GimfihiOptions::default(),
            }),
        ]
    }

    /// The submission a reader would send by opening `modal` and pressing
    /// submit without touching anything.
    #[requires(true)]
    #[ensures(true)]
    fn submit_unchanged(modal: &Modal) -> Submission {
        let mut values = BTreeMap::new();
        for component in modal.components.iter() {
            let ModalComponent::Label(labeled) = component else {
                continue;
            };
            let (custom_id, value) = match &labeled.control {
                ModalControl::TextInput(input) => (
                    input.custom_id.as_str().to_owned(),
                    SubmittedValue::Text(input.value.clone().unwrap_or_default()),
                ),
                ModalControl::RadioGroup(group) => (
                    group.custom_id.as_str().to_owned(),
                    SubmittedValue::Selected(
                        group
                            .options
                            .iter()
                            .filter(|option| option.default)
                            .map(|option| option.value.clone())
                            .collect(),
                    ),
                ),
                ModalControl::CheckboxGroup(group) => (
                    group.custom_id.as_str().to_owned(),
                    SubmittedValue::Selected(
                        group
                            .options
                            .iter()
                            .filter(|option| option.default)
                            .map(|option| option.value.clone())
                            .collect(),
                    ),
                ),
                ModalControl::StringSelect(select) => (
                    select.custom_id.as_str().to_owned(),
                    SubmittedValue::Selected(
                        select
                            .options
                            .iter()
                            .filter(|option| option.default)
                            .map(|option| option.value.clone())
                            .collect(),
                    ),
                ),
            };
            values.insert(custom_id, value);
        }
        Submission { values }
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_value(submission: &Submission, control: &str, value: SubmittedValue) -> Submission {
        let mut values = submission.values.clone();
        values.insert(control.to_owned(), value);
        Submission { values }
    }

    #[requires(true)]
    #[ensures(true)]
    fn selected(values: &[&str]) -> SubmittedValue {
        SubmittedValue::Selected(values.iter().map(|value| (*value).to_owned()).collect())
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_form_fits_the_platform_and_names_its_controls_once() {
        for request in every_request() {
            let tool = request.tool();
            let link = tool
                .has_web_page()
                .then_some("[Open in app](https://jbotci.app/x)");
            let modal = build(&published(request), link).expect("a form");
            assert!(
                modal.components.len() <= MAX_MODAL_COMPONENTS,
                "{tool}: {} components",
                modal.components.len()
            );
            assert!(utf16_len(&modal.title) <= MAX_MODAL_TITLE_UNITS, "{tool}");
            assert_eq!(
                modal.components.first().is_text_display(),
                tool.has_web_page(),
                "{tool}: the link is the first component, and only where there is a page"
            );
            let json = modal.to_json();
            assert!(
                json["custom_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("j1m.")),
                "{tool}"
            );
            let mut ids = Vec::new();
            for component in modal.components.iter() {
                if let ModalComponent::Label(labeled) = component {
                    let id = labeled.control.custom_id().as_str().to_owned();
                    assert!(!ids.contains(&id), "{tool}: {id} twice");
                    ids.push(id);
                }
                if let ModalComponent::Label(labeled) = component
                    && let ModalControl::StringSelect(select) = &labeled.control
                {
                    assert!(
                        select.options.len() <= MAX_SELECT_OPTIONS,
                        "{tool}: {} options",
                        select.options.len()
                    );
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_form_submitted_unchanged_means_the_request_it_was_opened_from() {
        for request in every_request().into_iter().chain(default_requests()) {
            let modal = build(&published(request.clone()), None).expect("a form");
            let parsed = parse_submission(&request, &submit_unchanged(&modal))
                .unwrap_or_else(|error| panic!("{}: {error}", request.tool()));
            assert_eq!(parsed, request, "{}", request.tool());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_optional_field_keeps_absent_empty_and_spacing_apart() {
        // Absent stays absent, an emptied field stays explicitly empty, and
        // whitespace someone typed is theirs to keep.
        let cases = [
            (None, "", None),
            (Some("(cbm)"), "", Some("")),
            (Some(""), "", Some("")),
            (None, "  ", Some("  ")),
            (Some("  "), "  ", Some("  ")),
            (Some("(cbm)"), " (cbm) ", Some(" (cbm) ")),
        ];
        for (previous, submitted, expected) in cases {
            let request = DiscordRequest::Gentufa(GentufaRequest {
                text: source("mi klama"),
                dialect: previous.map(source),
                options: GentufaOptions::default(),
            });
            let modal = build(&published(request.clone()), None).expect("a form");
            let submission = with_value(
                &submit_unchanged(&modal),
                ID_DIALECT,
                SubmittedValue::Text(submitted.to_owned()),
            );
            let DiscordRequest::Gentufa(parsed) =
                parse_submission(&request, &submission).expect("a request")
            else {
                panic!("a gentufa request");
            };
            assert_eq!(
                parsed.dialect.as_ref().map(SourceText::as_str),
                expected,
                "{previous:?} then {submitted:?}"
            );
        }

        // The same three states for the other optional fields.
        let jvozba = DiscordRequest::Jvozba(JvozbaRequest {
            parts: source("klama bajra"),
            rafsi: None,
            options: JvozbaOptions::default(),
        });
        let modal = build(&published(jvozba.clone()), None).expect("a form");
        let DiscordRequest::Jvozba(parsed) =
            parse_submission(&jvozba, &submit_unchanged(&modal)).expect("a request")
        else {
            panic!("a jvozba request");
        };
        assert_eq!(parsed.rafsi, None, "an absent rafsi field stays absent");

        let gimfihi = DiscordRequest::Gimfihi(GimfihiRequest {
            sources: None,
            options: GimfihiOptions::default(),
        });
        let modal = build(&published(gimfihi.clone()), None).expect("a form");
        let DiscordRequest::Gimfihi(parsed) =
            parse_submission(&gimfihi, &submit_unchanged(&modal)).expect("a request")
        else {
            panic!("a gimfi'i request");
        };
        assert_eq!(parsed.sources, None, "an absent source field stays absent");

        // A cukta query keeps its spacing rather than being filtered away.
        let cukta = DiscordRequest::Cukta(CuktaRequest {
            query: None,
            options: CuktaOptions {
                mode: CuktaMode::Contents,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        });
        let modal = build(&published(cukta.clone()), None).expect("a form");
        let spaced = with_value(
            &submit_unchanged(&modal),
            ID_QUERY,
            SubmittedValue::Text("  ".to_owned()),
        );
        let DiscordRequest::Cukta(parsed) = parse_submission(&cukta, &spaced).expect("a request")
        else {
            panic!("a cukta request");
        };
        assert_eq!(parsed.query.as_ref().map(SourceText::as_str), Some("  "));
        // A mode that needs a query is not satisfied by spaces.
        let searching = with_value(&spaced, ID_MODE, selected(&[CuktaMode::Word.slash_value()]));
        assert_eq!(
            parse_submission(&cukta, &searching),
            Err(SubmissionError::EmptyField {
                control: "query or reference"
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_malformed_or_stale_payload_is_refused_rather_than_applied() {
        let request = DiscordRequest::Gentufa(GentufaRequest {
            text: source("mi klama"),
            dialect: None,
            options: GentufaOptions::default(),
        });
        let modal = build(&published(request.clone()), None).expect("a form");
        let unchanged = submit_unchanged(&modal);

        // A value from another build is a stale form, not a setting.
        let stale = with_value(&unchanged, ID_FLAGS, selected(&["elided", "sparkles"]));
        assert!(matches!(
            parse_submission(&request, &stale),
            Err(SubmissionError::UnknownValue { .. })
        ));
        // The same choice twice is a payload to refuse.
        let repeated = with_value(&unchanged, ID_FLAGS, selected(&["elided", "elided"]));
        assert!(matches!(
            parse_submission(&request, &repeated),
            Err(SubmissionError::UnknownValue { .. })
        ));
        // A control answering in the wrong shape is not coerced.
        let wrong_shape = with_value(
            &unchanged,
            ID_TEXT,
            SubmittedValue::Selected(vec!["mi".to_owned()]),
        );
        assert_eq!(
            parse_submission(&request, &wrong_shape),
            Err(SubmissionError::WrongValueType { control: ID_TEXT })
        );
        let many = with_value(&unchanged, ID_VIEW, selected(&["tree", "brackets"]));
        assert_eq!(
            parse_submission(&request, &many),
            Err(SubmissionError::TooManyValues {
                control: ID_VIEW,
                selected: 2
            })
        );

        // And the reader itself refuses payloads a modal never sends.
        let duplicate = serde_json::json!({
            "components": [
                { "type": 18, "component": { "type": 4, "custom_id": "text", "value": "a" } },
                { "type": 18, "component": { "type": 4, "custom_id": "text", "value": "b" } }
            ]
        });
        assert_eq!(
            Submission::read(&duplicate),
            Err(SubmissionError::DuplicateControl {
                control: "text".to_owned()
            })
        );
        let not_text = serde_json::json!({
            "components": [
                { "type": 18, "component": { "type": 22, "custom_id": "flags", "values": [7] } }
            ]
        });
        assert_eq!(
            Submission::read(&not_text),
            Err(SubmissionError::NonTextValue {
                control: "flags".to_owned()
            })
        );
        let numeric = serde_json::json!({
            "components": [
                { "type": 18, "component": { "type": 4, "custom_id": "text", "value": 7 } }
            ]
        });
        assert!(matches!(
            Submission::read(&numeric),
            Err(SubmissionError::NonTextValue { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gimfihi_settings_that_change_the_candidates_start_the_list_over() {
        let request = |all_letters: bool, require_free_short_rafsi: bool, show_collisions: bool| {
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: Some(source("eng:5:go")),
                options: GimfihiOptions {
                    preset: None,
                    shapes: GismuShapeSet::empty().with(GismuShape::Ccvcv),
                    collisions: CollisionScope::All,
                    show_collisions,
                    all_letters,
                    require_free_short_rafsi,
                    page: PageNumber::new(4).expect("page"),
                },
            })
        };
        let base = request(false, false, false);
        let modal = build(&published(base.clone()), None).expect("a form");
        let unchanged = submit_unchanged(&modal);
        let page_of = |submission: &Submission| {
            let DiscordRequest::Gimfihi(parsed) =
                parse_submission(&base, submission).expect("a request")
            else {
                panic!("a gimfi'i request");
            };
            parsed.options.page.get()
        };

        assert_eq!(page_of(&unchanged), 4, "nothing changed, nothing moves");
        assert_eq!(
            page_of(&with_value(
                &unchanged,
                ID_OPTIONS,
                selected(&["shape-ccvcv", "collision-all", FLAG_ALL_LETTERS])
            )),
            1,
            "scoring all letters changes which candidates exist"
        );
        assert_eq!(
            page_of(&with_value(
                &unchanged,
                ID_OPTIONS,
                selected(&["shape-ccvcv", "collision-all", FLAG_FREE_RAFSI])
            )),
            1,
            "requiring a free short rafsi changes which candidates survive"
        );
        assert_eq!(
            page_of(&with_value(
                &unchanged,
                ID_OPTIONS,
                selected(&["shape-ccvcv", "collision-all", FLAG_SHOW_COLLISIONS])
            )),
            4,
            "showing collisions only changes how the list reads"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_combined_vlacku_selector_needs_exactly_one_page() {
        let request = DiscordRequest::Vlacku(new!(VlackuRequest {
            query: source("klama"),
            options: VlackuOptions::default(),
        }));
        let modal = build(&published(request.clone()), None).expect("a form");
        let unchanged = submit_unchanged(&modal);

        let two_pages = with_value(&unchanged, ID_PAGE_DETAILS, selected(&["p2", "p3"]));
        assert_eq!(
            parse_submission(&request, &two_pages),
            Err(SubmissionError::PageCount { selected: 2 })
        );
        let no_page = with_value(&unchanged, ID_PAGE_DETAILS, selected(&[DETAIL_DECOMPOSE]));
        assert_eq!(
            parse_submission(&request, &no_page),
            Err(SubmissionError::PageCount { selected: 0 })
        );

        // One page with details is read as both.
        let page_and_details = with_value(
            &unchanged,
            ID_PAGE_DETAILS,
            selected(&["p5", DETAIL_DECOMPOSE, DETAIL_ETYMOLOGY]),
        );
        let DiscordRequest::Vlacku(parsed) =
            parse_submission(&request, &page_and_details).expect("a request")
        else {
            panic!("a vlacku request");
        };
        assert_eq!(parsed.options.page.get(), 5);
        assert!(parsed.options.decompose_lujvo && parsed.options.show_etymology);

        // A page beyond what the selector offers is refused, not clamped.
        let beyond = with_value(&unchanged, ID_PAGE_DETAILS, selected(&["p99"]));
        assert!(matches!(
            parse_submission(&request, &beyond),
            Err(SubmissionError::UnknownValue { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_gimfihi_selector_needs_a_shape_and_one_collision_setting() {
        let request = DiscordRequest::Gimfihi(GimfihiRequest {
            sources: Some(source("eng:5:go")),
            options: GimfihiOptions::default(),
        });
        let modal = build(&published(request.clone()), None).expect("a form");
        let unchanged = submit_unchanged(&modal);

        let no_shape = with_value(&unchanged, ID_OPTIONS, selected(&["collision-all"]));
        assert_eq!(
            parse_submission(&request, &no_shape),
            Err(SubmissionError::ShapeCount)
        );
        let two_scopes = with_value(
            &unchanged,
            ID_OPTIONS,
            selected(&["shape-ccvcv", "collision-all", "collision-none"]),
        );
        assert_eq!(
            parse_submission(&request, &two_scopes),
            Err(SubmissionError::CollisionCount { selected: 2 })
        );
        let no_scope = with_value(&unchanged, ID_OPTIONS, selected(&["shape-ccvcv"]));
        assert_eq!(
            parse_submission(&request, &no_scope),
            Err(SubmissionError::CollisionCount { selected: 0 })
        );

        let valid = with_value(
            &unchanged,
            ID_OPTIONS,
            selected(&["shape-cvccv", "collision-none", FLAG_ALL_LETTERS]),
        );
        let DiscordRequest::Gimfihi(parsed) =
            parse_submission(&request, &valid).expect("a request")
        else {
            panic!("a gimfi'i request");
        };
        assert_eq!(parsed.options.shapes.shapes(), vec![GismuShape::Cvccv]);
        assert_eq!(parsed.options.collisions, CollisionScope::None);
        assert!(parsed.options.all_letters && !parsed.options.show_collisions);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn changing_the_source_or_a_filter_returns_to_the_first_page() {
        let request = DiscordRequest::Vlacku(new!(VlackuRequest {
            query: source("klama"),
            options: VlackuOptions {
                page: PageNumber::new(6).expect("page"),
                ..VlackuOptions::default()
            },
        }));
        let modal = build(&published(request.clone()), None).expect("a form");
        let unchanged = submit_unchanged(&modal);

        // The same query on the same page keeps the page.
        let DiscordRequest::Vlacku(kept) =
            parse_submission(&request, &unchanged).expect("a request")
        else {
            panic!("a vlacku request");
        };
        assert_eq!(kept.options.page.get(), 6);

        // A new query starts over, even though page six is still selected.
        let new_query = with_value(
            &unchanged,
            ID_QUERY,
            SubmittedValue::Text("bajra".to_owned()),
        );
        let DiscordRequest::Vlacku(reset) =
            parse_submission(&request, &new_query).expect("a request")
        else {
            panic!("a vlacku request");
        };
        assert_eq!(reset.options.page.get(), 1, "a new query starts over");

        // So does a new filter.
        let new_filter = with_value(&unchanged, ID_WORD_TYPES, selected(&["gismu"]));
        let DiscordRequest::Vlacku(reset) =
            parse_submission(&request, &new_filter).expect("a request")
        else {
            panic!("a vlacku request");
        };
        assert_eq!(reset.options.page.get(), 1);

        // Cukta and gimfi'i behave the same way.
        let cukta = DiscordRequest::Cukta(CuktaRequest {
            query: Some(source("tanru")),
            options: CuktaOptions {
                mode: CuktaMode::Word,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::new(5).expect("page"),
            },
        });
        let modal = build(&published(cukta.clone()), None).expect("a form");
        let changed = with_value(
            &submit_unchanged(&modal),
            ID_KINDS,
            selected(&["paragraph"]),
        );
        let DiscordRequest::Cukta(parsed) = parse_submission(&cukta, &changed).expect("a request")
        else {
            panic!("a cukta request");
        };
        assert_eq!(parsed.options.page.get(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_cleared_optional_field_stays_cleared_and_an_empty_required_one_is_refused() {
        let request = DiscordRequest::Gentufa(GentufaRequest {
            text: source("mi klama"),
            dialect: Some(source("(cbm)")),
            options: GentufaOptions::default(),
        });
        let modal = build(&published(request.clone()), None).expect("a form");
        let unchanged = submit_unchanged(&modal);

        let cleared = with_value(&unchanged, ID_DIALECT, SubmittedValue::Text(String::new()));
        let DiscordRequest::Gentufa(parsed) =
            parse_submission(&request, &cleared).expect("a request")
        else {
            panic!("a gentufa request");
        };
        assert_eq!(
            parsed.dialect.as_ref().map(SourceText::as_str),
            Some(""),
            "an emptied dialect stays explicit"
        );

        let blank = with_value(&unchanged, ID_TEXT, SubmittedValue::Text("   ".to_owned()));
        assert_eq!(
            parse_submission(&request, &blank),
            Err(SubmissionError::EmptyField { control: "text" })
        );

        // A value beyond the source limit is refused with its size.
        let oversize = with_value(
            &unchanged,
            ID_TEXT,
            SubmittedValue::Text("a".repeat(MAX_SOURCE_UNITS + 5)),
        );
        assert_eq!(
            parse_submission(&request, &oversize),
            Err(SubmissionError::Oversize {
                control: "text",
                units: MAX_SOURCE_UNITS + 5,
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn values_are_read_out_of_a_real_submission_payload() {
        // The shape Discord sends back: labels wrapping their controls.
        let data = serde_json::json!({
            "custom_id": "j1m.g.1.123",
            "components": [
                { "type": 18, "component": { "type": 4, "custom_id": "text", "value": "mi klama" } },
                { "type": 18, "component": { "type": 4, "custom_id": "dialect", "value": "" } },
                { "type": 18, "component": { "type": 21, "custom_id": "view", "value": "tree" } },
                { "type": 18, "component": { "type": 22, "custom_id": "flags", "values": ["elided", "glosses"] } }
            ]
        });
        let submission = Submission::read(&data).expect("a well-formed payload");
        let request = DiscordRequest::Gentufa(GentufaRequest {
            text: source("old"),
            dialect: None,
            options: GentufaOptions::default(),
        });
        let DiscordRequest::Gentufa(parsed) =
            parse_submission(&request, &submission).expect("a request")
        else {
            panic!("a gentufa request");
        };
        assert_eq!(parsed.text.as_str(), "mi klama");
        assert_eq!(
            parsed.dialect, None,
            "an empty control over an absent field stays absent"
        );
        // The same payload over a request that had a dialect clears it.
        let had_dialect = DiscordRequest::Gentufa(GentufaRequest {
            text: source("old"),
            dialect: Some(source("(cbm)")),
            options: GentufaOptions::default(),
        });
        let DiscordRequest::Gentufa(cleared) =
            parse_submission(&had_dialect, &submission).expect("a request")
        else {
            panic!("a gentufa request");
        };
        assert_eq!(cleared.dialect.as_ref().map(SourceText::as_str), Some(""));
        assert_eq!(parsed.options.view, GentufaTextView::Tree);
        assert!(parsed.options.show_elided && parsed.options.show_glosses);
        assert!(!parsed.options.include_diagram && !parsed.options.show_compounds);

        // A radio group with nothing chosen is reported, not guessed.
        let empty = serde_json::json!({
            "components": [
                { "type": 18, "component": { "type": 4, "custom_id": "text", "value": "mi" } },
                { "type": 18, "component": { "type": 21, "custom_id": "view", "value": null } },
                { "type": 18, "component": { "type": 22, "custom_id": "flags", "values": [] } }
            ]
        });
        assert_eq!(
            parse_submission(
                &request,
                &Submission::read(&empty).expect("a well-formed payload")
            ),
            Err(SubmissionError::MissingControl { control: "view" })
        );
    }
}

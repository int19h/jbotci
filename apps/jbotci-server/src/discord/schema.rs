//! The authoritative `/jbotci` slash-command schema.
//!
//! Registration payloads are generated from this schema and interaction
//! payloads are decoded against it, so the two cannot drift. Slash arguments
//! describe only the task (source text, query, minimal task selector);
//! presentation lives in the customization modal.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde_json::{Value, json};

use super::request::{
    CuktaMode, CuktaOptions, CuktaRequest, CuktaResultKindSet, DiscordRequest, DiscordTool,
    GentufaOptions, GentufaRequest, GimfihiOptions, GimfihiRequest, JvozbaOptions, JvozbaRequest,
    JvozbaTarget, MAX_SOURCE_UNITS, OversizeSource, PageNumber, SourceText, VlackuMode,
    VlackuOptions, VlackuRequest, VlaseiOptions, VlaseiRequest, VlataiOptions, VlataiRequest,
};

pub(crate) const COMMAND_NAME: &str = "jbotci";
pub(crate) const COMMAND_DESCRIPTION: &str = "Lojban tools";

/// Discord application command option types used by the schema.
const OPTION_TYPE_SUBCOMMAND: u8 = 1;
const OPTION_TYPE_STRING: u8 = 3;

/// Discord caps option names at 32 and descriptions at 100 characters.
const MAX_OPTION_NAME_LEN: usize = 32;
const MAX_DESCRIPTION_LEN: usize = 100;

/// Where a choice option takes its values from. Choices are derived from the
/// request enums at registration time, so the decoder's `from_slash_value`
/// and the registered choice list are the same set by construction.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceSource {
    VlackuMode,
    CuktaMode,
    JvozbaTarget,
    GimfihiPreset,
}

impl ChoiceSource {
    /// `(name, value)` pairs in registration order.
    #[requires(true)]
    #[ensures(!ret.is_empty() && ret.len() <= 25)]
    pub(crate) fn choices(self) -> Vec<(String, String)> {
        match self {
            Self::VlackuMode => VlackuMode::ALL
                .iter()
                .map(|mode| (mode.label().to_owned(), mode.slash_value().to_owned()))
                .collect(),
            Self::CuktaMode => CuktaMode::ALL
                .iter()
                .map(|mode| (mode.label().to_owned(), mode.slash_value().to_owned()))
                .collect(),
            Self::JvozbaTarget => [JvozbaTarget::Lujvo, JvozbaTarget::Cmevla]
                .iter()
                .map(|target| {
                    (
                        target.slash_value().to_owned(),
                        target.slash_value().to_owned(),
                    )
                })
                .collect(),
            Self::GimfihiPreset => jbotci_gimfihi::all_presets()
                .iter()
                .map(|preset| (preset.as_str().to_owned(), preset.as_str().to_owned()))
                .collect(),
        }
    }
}

#[invariant(::Choice(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlashOptionKind {
    /// Free text bounded by [`MAX_SOURCE_UNITS`].
    Text,
    Choice(ChoiceSource),
}

#[invariant(!name.is_empty() && name.len() <= MAX_OPTION_NAME_LEN && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'-'))]
#[invariant(!description.is_empty() && description.chars().count() <= MAX_DESCRIPTION_LEN)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlashOption {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) kind: SlashOptionKind,
    pub(crate) required: bool,
}

#[invariant(!description.is_empty() && description.chars().count() <= MAX_DESCRIPTION_LEN)]
#[invariant(options.windows(2).all(|pair| pair[0].required || !pair[1].required), "Discord requires required options before optional ones")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlashSubcommand {
    pub(crate) tool: DiscordTool,
    pub(crate) description: &'static str,
    pub(crate) options: Vec<SlashOption>,
}

#[requires(!name.is_empty())]
#[requires(!description.is_empty())]
#[ensures(ret.name == name && ret.required == required)]
fn text_option(name: &'static str, description: &'static str, required: bool) -> SlashOption {
    bityzba::new!(SlashOption {
        name,
        description,
        kind: SlashOptionKind::Text,
        required,
    })
}

#[requires(!name.is_empty())]
#[requires(!description.is_empty())]
#[ensures(ret.name == name && !ret.required)]
fn choice_option(
    name: &'static str,
    description: &'static str,
    source: ChoiceSource,
) -> SlashOption {
    bityzba::new!(SlashOption {
        name,
        description,
        kind: SlashOptionKind::Choice(source),
        required: false,
    })
}

/// The subcommand definitions, one per tool, in registration order.
#[requires(true)]
#[ensures(ret.len() == DiscordTool::ALL.len())]
#[ensures(ret.iter().zip(DiscordTool::ALL).all(|(subcommand, tool)| subcommand.tool == tool))]
pub(crate) fn subcommands() -> Vec<SlashSubcommand> {
    DiscordTool::ALL
        .into_iter()
        .map(|tool| {
            let (description, options) = match tool {
                DiscordTool::Gentufa => (
                    "Parse Lojban syntax; the result starts as brackets with diagnostics",
                    vec![text_option("text", "Lojban text to parse", true)],
                ),
                DiscordTool::Vlasei => (
                    "Split Lojban text into words with the morphology parser",
                    vec![text_option("text", "Lojban text to segment", true)],
                ),
                DiscordTool::Vlatai => (
                    "Check one Lojban word: validity, class, phonemes, formation",
                    vec![text_option("text", "One Lojban word", true)],
                ),
                DiscordTool::Vlacku => (
                    "Search the Lojban dictionary",
                    vec![
                        text_option(
                            "query",
                            "Word, rafsi, lujvo, sound, or meaning to look up",
                            true,
                        ),
                        choice_option(
                            "mode",
                            "What the query matches (default: word)",
                            ChoiceSource::VlackuMode,
                        ),
                    ],
                ),
                DiscordTool::Cukta => (
                    "Read or search the CLL reference grammar",
                    vec![
                        text_option(
                            "query",
                            "Search text or a section/example reference such as 5.2",
                            false,
                        ),
                        choice_option(
                            "mode",
                            "Meaning search with a query, contents without one, unless set",
                            ChoiceSource::CuktaMode,
                        ),
                    ],
                ),
                DiscordTool::Jvozba => (
                    "Build a lujvo or cmevla from source words",
                    vec![
                        text_option(
                            "parts",
                            "Pieces in order; a rafsi as given goes in hyphens: blanu -blo- zdani",
                            true,
                        ),
                        choice_option(
                            "mode",
                            "Word kind to build (default: lujvo)",
                            ChoiceSource::JvozbaTarget,
                        ),
                    ],
                ),
                DiscordTool::Gimfihi => (
                    "Propose gismu candidates from source-language words",
                    vec![
                        text_option(
                            "sources",
                            "LANG[:WEIGHT]:WORD records separated by commas; WORD may be [IPA]",
                            false,
                        ),
                        choice_option(
                            "preset",
                            "Weight preset supplying the source languages",
                            ChoiceSource::GimfihiPreset,
                        ),
                    ],
                ),
            };
            bityzba::new!(SlashSubcommand {
                tool,
                description,
                options,
            })
        })
        .collect()
}

/// The registration body for `PUT /applications/{id}/commands`.
#[requires(true)]
#[ensures(ret.get("name").and_then(Value::as_str) == Some(COMMAND_NAME))]
pub(crate) fn registration_payload() -> Value {
    json!({
        "name": COMMAND_NAME,
        "type": 1,
        "description": COMMAND_DESCRIPTION,
        // The command is installed both to guilds and to users and works in
        // direct messages, which is what the application is configured for
        // and what the live registration already had. Discord documents these
        // as defaulting to the application's configured contexts, so stating
        // them does not widen anything: it pins the intended availability
        // here rather than to a portal setting that may change.
        "integration_types": [0, 1],
        "contexts": [0, 1, 2],
        "dm_permission": true,
        "options": subcommands()
            .iter()
            .map(|subcommand| json!({
                "name": subcommand.tool.name(),
                "description": subcommand.description,
                "type": OPTION_TYPE_SUBCOMMAND,
                "options": subcommand.options.iter().map(option_payload).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
    })
}

#[requires(true)]
#[ensures(ret.get("name").and_then(Value::as_str) == Some(option.name))]
fn option_payload(option: &SlashOption) -> Value {
    let mut payload = json!({
        "name": option.name,
        "description": option.description,
        "type": OPTION_TYPE_STRING,
        "required": option.required,
    });
    match option.kind {
        SlashOptionKind::Text => {
            // Discord enforces these bounds in the client before the
            // interaction is sent, so oversize input normally never reaches
            // the server; the decoder still enforces the same bound.
            payload["max_length"] = json!(MAX_SOURCE_UNITS);
            if option.required {
                payload["min_length"] = json!(1);
            }
        }
        SlashOptionKind::Choice(source) => {
            payload["choices"] = Value::Array(
                source
                    .choices()
                    .into_iter()
                    .map(|(name, value)| json!({ "name": name, "value": value }))
                    .collect(),
            );
        }
    }
    payload
}

/// Why an application-command payload could not be decoded. Messages are
/// user-facing.
#[invariant(::NotJbotci { .. } => true)]
#[invariant(::UnknownSubcommand { .. } => true)]
#[invariant(::UnknownOption { .. } => true)]
#[invariant(::DuplicateOption { .. } => true)]
#[invariant(::MissingRequiredOption { .. } => true)]
#[invariant(::WrongOptionType { .. } => true)]
#[invariant(::InvalidChoice { .. } => true)]
#[invariant(::EmptyText { .. } => true)]
#[invariant(::OversizeText { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandDecodeError {
    NotJbotci {
        name: String,
    },
    MissingSubcommand,
    UnknownSubcommand {
        name: String,
    },
    /// An option this schema does not define. Old registered definitions can
    /// still deliver `format`, `count` and friends for a while; the user
    /// needs to rerun without them.
    UnknownOption {
        tool: DiscordTool,
        option: String,
    },
    DuplicateOption {
        tool: DiscordTool,
        option: &'static str,
    },
    MissingRequiredOption {
        tool: DiscordTool,
        option: &'static str,
    },
    WrongOptionType {
        tool: DiscordTool,
        option: &'static str,
    },
    InvalidChoice {
        tool: DiscordTool,
        option: &'static str,
        value: String,
    },
    EmptyText {
        tool: DiscordTool,
        option: &'static str,
    },
    OversizeText {
        tool: DiscordTool,
        option: &'static str,
        error: OversizeSource,
    },
}

impl fmt::Display for CommandDecodeError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotJbotci { name } => write!(formatter, "Unknown command `/{name}`."),
            Self::MissingSubcommand => formatter.write_str("Missing jbotci subcommand."),
            Self::UnknownSubcommand { name } => {
                write!(formatter, "Unknown jbotci subcommand `{name}`.")
            }
            Self::UnknownOption { tool, option } => write!(
                formatter,
                "The `{option}` option is no longer part of `/jbotci {tool}`. Rerun the command without it; presentation choices now live in the ⚙️ form."
            ),
            Self::DuplicateOption { tool, option } => {
                write!(formatter, "`/jbotci {tool}` received `{option}` twice.")
            }
            Self::MissingRequiredOption { tool, option } => {
                write!(formatter, "`/jbotci {tool}` needs the `{option}` option.")
            }
            Self::WrongOptionType { tool, option } => {
                write!(formatter, "`/jbotci {tool}`: `{option}` must be text.")
            }
            Self::InvalidChoice {
                tool,
                option,
                value,
            } => write!(
                formatter,
                "`/jbotci {tool}`: `{value}` is not a valid `{option}` choice."
            ),
            Self::EmptyText { tool, option } => {
                write!(formatter, "`/jbotci {tool}`: `{option}` must not be blank.")
            }
            Self::OversizeText {
                tool,
                option,
                error,
            } => write!(
                formatter,
                "`/jbotci {tool}`: `{option}` is too long ({error})."
            ),
        }
    }
}

impl std::error::Error for CommandDecodeError {}

/// Decode the `data` object of an APPLICATION_COMMAND interaction.
#[requires(true)]
#[ensures(true)]
pub(crate) fn decode_command(data: &Value) -> Result<DiscordRequest, CommandDecodeError> {
    let name = data.get("name").and_then(Value::as_str).unwrap_or_default();
    if name != COMMAND_NAME {
        return Err(CommandDecodeError::NotJbotci {
            name: name.to_owned(),
        });
    }
    let subcommand_value = data
        .get("options")
        .and_then(Value::as_array)
        .and_then(|options| options.first())
        .ok_or(CommandDecodeError::MissingSubcommand)?;
    let subcommand_name = subcommand_value
        .get("name")
        .and_then(Value::as_str)
        .ok_or(CommandDecodeError::MissingSubcommand)?;
    let tool = DiscordTool::from_name(subcommand_name).ok_or_else(|| {
        CommandDecodeError::UnknownSubcommand {
            name: subcommand_name.to_owned(),
        }
    })?;
    let schema = subcommands()
        .into_iter()
        .find(|subcommand| subcommand.tool == tool)
        .expect("every tool has a subcommand schema");
    let provided = subcommand_value
        .get("options")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let values = DecodedOptions::collect(&schema, provided)?;
    build_request(tool, &values)
}

/// Option values matched against the schema.
#[invariant(values.iter().enumerate().all(|(index, (name, _))| !values[..index].iter().any(|(seen, _)| seen == name)), "each option is decoded once")]
struct DecodedOptions<'a> {
    tool: DiscordTool,
    values: Vec<(&'static str, &'a str)>,
}

impl<'a> DecodedOptions<'a> {
    #[requires(true)]
    #[ensures(true)]
    fn collect(
        schema: &SlashSubcommand,
        provided: &'a [Value],
    ) -> Result<Self, CommandDecodeError> {
        let tool = schema.tool;
        let mut values: Vec<(&'static str, &'a str)> = Vec::new();
        for option in provided {
            let name = option
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(definition) = schema
                .options
                .iter()
                .find(|candidate| candidate.name == name)
            else {
                return Err(CommandDecodeError::UnknownOption {
                    tool,
                    option: name.to_owned(),
                });
            };
            if values.iter().any(|(seen, _)| *seen == definition.name) {
                return Err(CommandDecodeError::DuplicateOption {
                    tool,
                    option: definition.name,
                });
            }
            let value = option.get("value").and_then(Value::as_str).ok_or(
                CommandDecodeError::WrongOptionType {
                    tool,
                    option: definition.name,
                },
            )?;
            if let SlashOptionKind::Choice(source) = definition.kind
                && !source.choices().iter().any(|(_, choice)| choice == value)
            {
                return Err(CommandDecodeError::InvalidChoice {
                    tool,
                    option: definition.name,
                    value: value.to_owned(),
                });
            }
            values.push((definition.name, value));
        }
        for definition in schema.options.iter().filter(|option| option.required) {
            if !values.iter().any(|(seen, _)| *seen == definition.name) {
                return Err(CommandDecodeError::MissingRequiredOption {
                    tool,
                    option: definition.name,
                });
            }
        }
        Ok(bityzba::new!(DecodedOptions { tool, values }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw(&self, name: &'static str) -> Option<&'a str> {
        self.values
            .iter()
            .find(|(seen, _)| *seen == name)
            .map(|(_, value)| *value)
    }

    /// A text option that must carry visible content when present.
    #[requires(true)]
    #[ensures(true)]
    fn text(&self, name: &'static str) -> Result<Option<SourceText>, CommandDecodeError> {
        let Some(value) = self.raw(name) else {
            return Ok(None);
        };
        if value.trim().is_empty() {
            return Err(CommandDecodeError::EmptyText {
                tool: self.tool,
                option: name,
            });
        }
        SourceText::new(value)
            .map(Some)
            .map_err(|error| CommandDecodeError::OversizeText {
                tool: self.tool,
                option: name,
                error,
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn required_text(&self, name: &'static str) -> Result<SourceText, CommandDecodeError> {
        self.text(name)?
            .ok_or(CommandDecodeError::MissingRequiredOption {
                tool: self.tool,
                option: name,
            })
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|request| request.tool() == tool) || ret.is_err())]
fn build_request(
    tool: DiscordTool,
    values: &DecodedOptions<'_>,
) -> Result<DiscordRequest, CommandDecodeError> {
    Ok(match tool {
        DiscordTool::Gentufa => DiscordRequest::Gentufa(GentufaRequest {
            text: values.required_text("text")?,
            dialect: None,
            options: GentufaOptions::default(),
        }),
        DiscordTool::Vlasei => DiscordRequest::Vlasei(VlaseiRequest {
            text: values.required_text("text")?,
            dialect: None,
            options: VlaseiOptions::default(),
        }),
        DiscordTool::Vlatai => DiscordRequest::Vlatai(VlataiRequest {
            text: values.required_text("text")?,
            dialect: None,
            options: VlataiOptions::default(),
        }),
        DiscordTool::Vlacku => {
            let mode = values
                .raw("mode")
                .map(|value| {
                    VlackuMode::from_slash_value(value)
                        .expect("choice was validated against the schema")
                })
                .unwrap_or_default();
            let query = values.required_text("query")?;
            DiscordRequest::Vlacku(bityzba::new!(VlackuRequest {
                query,
                options: VlackuOptions {
                    mode,
                    ..VlackuOptions::default()
                },
            }))
        }
        DiscordTool::Cukta => {
            let query = values.text("query")?;
            let explicit_mode = values.raw("mode").map(|value| {
                CuktaMode::from_slash_value(value).expect("choice was validated against the schema")
            });
            // PM decision: an explicit mode always wins; otherwise a query
            // means meaning search and no query means the book's contents.
            let mode = explicit_mode.unwrap_or(if query.is_some() {
                CuktaMode::Meaning
            } else {
                CuktaMode::Contents
            });
            // A mode without its query is an incomplete task, published with
            // its ⚙️ form so the reader can finish it; it is not a command
            // this build cannot read.
            DiscordRequest::Cukta(CuktaRequest {
                query,
                options: CuktaOptions {
                    mode,
                    kinds: CuktaResultKindSet::empty(),
                    page: PageNumber::first(),
                },
            })
        }
        DiscordTool::Jvozba => {
            let target = values
                .raw("mode")
                .map(|value| {
                    JvozbaTarget::from_slash_value(value)
                        .expect("choice was validated against the schema")
                })
                .unwrap_or_default();
            DiscordRequest::Jvozba(JvozbaRequest {
                parts: values.required_text("parts")?,
                options: JvozbaOptions { target },
            })
        }
        DiscordTool::Gimfihi => {
            let preset = values.raw("preset").map(|value| {
                jbotci_gimfihi::parse_preset(value)
                    .expect("choice was validated against the schema")
            });
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: values.text("sources")?,
                options: GimfihiOptions {
                    preset,
                    ..GimfihiOptions::default()
                },
            })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::{GentufaTextView, GimfihiPreset, VlaseiView};

    #[requires(!name.is_empty())]
    #[ensures(ret.is_object())]
    fn option(name: &str, value: &str) -> Value {
        json!({ "name": name, "type": 3, "value": value })
    }

    #[requires(!subcommand.is_empty())]
    #[ensures(ret.is_object())]
    fn command_data(subcommand: &str, options: Vec<Value>) -> Value {
        json!({
            "name": COMMAND_NAME,
            "type": 1,
            "options": [{ "name": subcommand, "type": 1, "options": options }]
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn registration_covers_the_seven_tools_with_minimal_task_arguments() {
        let payload = registration_payload();
        let subcommands = payload["options"].as_array().expect("subcommands");
        let names = subcommands
            .iter()
            .map(|subcommand| subcommand["name"].as_str().expect("name"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "gentufa", "vlasei", "vlatai", "vlacku", "cukta", "jvozba", "gimfihi"
            ]
        );
        let shapes = subcommands
            .iter()
            .map(|subcommand| {
                subcommand["options"]
                    .as_array()
                    .expect("options")
                    .iter()
                    .map(|option| {
                        (
                            option["name"].as_str().expect("option name"),
                            option["required"].as_bool().expect("required"),
                            option.get("choices").is_some(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            shapes,
            vec![
                vec![("text", true, false)],
                vec![("text", true, false)],
                vec![("text", true, false)],
                vec![("query", true, false), ("mode", false, true)],
                vec![("query", false, false), ("mode", false, true)],
                vec![("parts", true, false), ("mode", false, true)],
                vec![("sources", false, false), ("preset", false, true)],
            ]
        );
        for subcommand in subcommands {
            assert!(
                subcommand["description"]
                    .as_str()
                    .expect("description")
                    .chars()
                    .count()
                    <= 100
            );
            for option in subcommand["options"].as_array().expect("options") {
                assert!(
                    option["description"]
                        .as_str()
                        .expect("description")
                        .chars()
                        .count()
                        <= 100
                );
                assert_eq!(option["type"], 3);
                if option.get("choices").is_none() {
                    assert_eq!(option["max_length"], MAX_SOURCE_UNITS);
                    assert_eq!(
                        option.get("min_length").is_some(),
                        option["required"] == true
                    );
                } else {
                    assert!(option["choices"].as_array().expect("choices").len() <= 25);
                }
                // None of the removed presentation switches are advertised.
                assert!(
                    ![
                        "format",
                        "count",
                        "word-type",
                        "show-elided",
                        "show-glosses",
                        "show-compounds",
                        "dialect",
                        "png"
                    ]
                    .contains(&option["name"].as_str().expect("name"))
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn choice_lists_match_the_decoder_enums() {
        let vlacku = ChoiceSource::VlackuMode.choices();
        assert_eq!(
            vlacku
                .iter()
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>(),
            ["word", "rafsi", "lujvo", "sound", "meaning"]
        );
        for (_, value) in &vlacku {
            assert!(VlackuMode::from_slash_value(value).is_some());
        }
        let cukta = ChoiceSource::CuktaMode.choices();
        assert_eq!(
            cukta
                .iter()
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>(),
            ["meaning", "word", "section", "example", "contents"]
        );
        let presets = ChoiceSource::GimfihiPreset.choices();
        assert_eq!(presets.len(), jbotci_gimfihi::all_presets().len());
        for (_, value) in &presets {
            assert!(jbotci_gimfihi::parse_preset(value).is_ok());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decodes_each_tool_with_epic_defaults() {
        let DiscordRequest::Gentufa(gentufa) =
            decode_command(&command_data("gentufa", vec![option("text", "mi klama")]))
                .expect("gentufa")
        else {
            panic!("gentufa request");
        };
        assert_eq!(gentufa.text.as_str(), "mi klama");
        assert_eq!(gentufa.dialect, None);
        assert_eq!(gentufa.options.view, GentufaTextView::Brackets);
        assert!(!gentufa.options.include_diagram);
        assert!(gentufa.options.show_compounds);
        assert!(!gentufa.options.show_glosses && !gentufa.options.show_elided);

        let DiscordRequest::Vlasei(vlasei) =
            decode_command(&command_data("vlasei", vec![option("text", "coi ro do")]))
                .expect("vlasei")
        else {
            panic!("vlasei request");
        };
        assert_eq!(vlasei.options.view, VlaseiView::Words);

        let DiscordRequest::Vlatai(vlatai) =
            decode_command(&command_data("vlatai", vec![option("text", "klama")])).expect("vlatai")
        else {
            panic!("vlatai request");
        };
        assert!(
            vlatai.options.mark_stress && vlatai.options.mark_glides && vlatai.options.show_details
        );

        let DiscordRequest::Vlacku(vlacku) = decode_command(&command_data(
            "vlacku",
            vec![option("query", "klama"), option("mode", "rafsi")],
        ))
        .expect("vlacku") else {
            panic!("vlacku request");
        };
        assert_eq!(vlacku.options.mode, VlackuMode::Rafsi);
        assert_eq!(vlacku.options.page.get(), 1);
        assert!(vlacku.options.word_types.is_empty());

        let DiscordRequest::Jvozba(jvozba) = decode_command(&command_data(
            "jvozba",
            vec![
                option("parts", "klama -kla- bajra"),
                option("mode", "cmevla"),
            ],
        ))
        .expect("jvozba") else {
            panic!("jvozba request");
        };
        assert_eq!(
            jvozba.parts.as_str(),
            "klama -kla- bajra",
            "one field carries words and literal rafsi in the order given"
        );
        assert_eq!(jvozba.options.target, JvozbaTarget::Cmevla);

        let DiscordRequest::Gimfihi(gimfihi) =
            decode_command(&command_data("gimfihi", vec![option("preset", "ilmen12")]))
                .expect("gimfihi")
        else {
            panic!("gimfihi request");
        };
        assert_eq!(gimfihi.sources, None);
        assert_eq!(gimfihi.options.preset, Some(GimfihiPreset::Ilmen12));
        assert_eq!(
            gimfihi.options.collisions,
            jbotci_gimfihi::CollisionScope::All
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_mode_resolution_follows_the_pm_decision() {
        let DiscordRequest::Cukta(contents) =
            decode_command(&command_data("cukta", vec![])).expect("contents")
        else {
            panic!("cukta request");
        };
        assert_eq!(contents.options.mode, CuktaMode::Contents);
        assert_eq!(contents.query, None);

        let DiscordRequest::Cukta(meaning) =
            decode_command(&command_data("cukta", vec![option("query", "tanru")]))
                .expect("meaning")
        else {
            panic!("cukta request");
        };
        assert_eq!(meaning.options.mode, CuktaMode::Meaning);

        let DiscordRequest::Cukta(section) = decode_command(&command_data(
            "cukta",
            vec![option("query", "5.2"), option("mode", "section")],
        ))
        .expect("section") else {
            panic!("cukta request");
        };
        assert_eq!(section.options.mode, CuktaMode::Section);

        // A search mode with no query is recorded as asked: the mode stays,
        // the query stays absent, and running it is what reports the gap.
        let DiscordRequest::Cukta(incomplete) =
            decode_command(&command_data("cukta", vec![option("mode", "word")]))
                .expect("an incomplete task is still a request")
        else {
            panic!("cukta request");
        };
        assert_eq!(incomplete.options.mode, CuktaMode::Word);
        assert_eq!(incomplete.query, None);

        // An explicit contents request with a query keeps the query but reads
        // the contents; the query is not silently discarded from the state.
        let DiscordRequest::Cukta(explicit) = decode_command(&command_data(
            "cukta",
            vec![option("query", "tanru"), option("mode", "contents")],
        ))
        .expect("explicit contents") else {
            panic!("cukta request");
        };
        assert_eq!(explicit.options.mode, CuktaMode::Contents);
        assert_eq!(
            explicit.query.as_ref().map(SourceText::as_str),
            Some("tanru")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn removed_and_malformed_options_fail_explicitly() {
        let error = decode_command(&command_data(
            "gentufa",
            vec![option("text", "mi klama"), option("format", "png")],
        ))
        .expect_err("format was removed");
        assert_eq!(
            error,
            CommandDecodeError::UnknownOption {
                tool: DiscordTool::Gentufa,
                option: "format".to_owned()
            }
        );
        assert!(
            error
                .to_string()
                .contains("no longer part of `/jbotci gentufa`")
        );

        assert_eq!(
            decode_command(&command_data(
                "vlacku",
                vec![option("query", "x"), option("count", "5")]
            )),
            Err(CommandDecodeError::UnknownOption {
                tool: DiscordTool::Vlacku,
                option: "count".to_owned()
            })
        );
        assert_eq!(
            decode_command(&command_data("vlacku", vec![option("mode", "word")])),
            Err(CommandDecodeError::MissingRequiredOption {
                tool: DiscordTool::Vlacku,
                option: "query"
            })
        );
        assert_eq!(
            decode_command(&command_data(
                "vlacku",
                vec![option("query", "x"), option("mode", "regex")]
            )),
            Err(CommandDecodeError::InvalidChoice {
                tool: DiscordTool::Vlacku,
                option: "mode",
                value: "regex".to_owned()
            })
        );
        assert_eq!(
            decode_command(&command_data("gentufa", vec![option("text", "   ")])),
            Err(CommandDecodeError::EmptyText {
                tool: DiscordTool::Gentufa,
                option: "text"
            })
        );
        assert_eq!(
            decode_command(&command_data(
                "gentufa",
                vec![option("text", "a"), option("text", "b")]
            )),
            Err(CommandDecodeError::DuplicateOption {
                tool: DiscordTool::Gentufa,
                option: "text"
            })
        );
        assert_eq!(
            decode_command(&command_data("tersmu", vec![])),
            Err(CommandDecodeError::UnknownSubcommand {
                name: "tersmu".to_owned()
            })
        );
        assert_eq!(
            decode_command(&json!({ "name": "other", "type": 1, "options": [] })),
            Err(CommandDecodeError::NotJbotci {
                name: "other".to_owned()
            })
        );
        let wrong_type = json!({
            "name": COMMAND_NAME,
            "type": 1,
            "options": [{ "name": "gentufa", "type": 1, "options": [{ "name": "text", "type": 4, "value": 7 }] }]
        });
        assert_eq!(
            decode_command(&wrong_type),
            Err(CommandDecodeError::WrongOptionType {
                tool: DiscordTool::Gentufa,
                option: "text"
            })
        );
        let long = "a".repeat(MAX_SOURCE_UNITS + 1);
        assert!(matches!(
            decode_command(&command_data("gentufa", vec![option("text", &long)])),
            Err(CommandDecodeError::OversizeText {
                tool: DiscordTool::Gentufa,
                option: "text",
                ..
            })
        ));
    }
}

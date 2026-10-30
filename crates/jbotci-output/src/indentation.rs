//! The width of one indentation level, bounded before rendering.

use std::{fmt, str::FromStr};

use bityzba::{invariant, new, requires};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Spaces per indentation level. Zero selects compact output.
#[invariant(*spaces <= Indentation::MAX_SPACES, "indentation must be between 0 and 8 spaces")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Indentation {
    spaces: usize,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("indentation must be between 0 and 8 spaces")]
pub struct InvalidIndentation;

impl Indentation {
    pub const MAX_SPACES: usize = 8;

    #[requires(true)]
    #[ensures(ret.spaces() == 0)]
    pub fn compact() -> Self {
        new!(Indentation { spaces: 0 })
    }

    #[requires(true)]
    #[ensures(ret <= Self::MAX_SPACES)]
    pub fn spaces(self) -> usize {
        self.spaces
    }
}

impl Default for Indentation {
    #[requires(true)]
    #[ensures(ret.spaces() == 2)]
    fn default() -> Self {
        new!(Indentation { spaces: 2 })
    }
}

impl TryFrom<usize> for Indentation {
    type Error = InvalidIndentation;

    #[requires(true)]
    #[ensures(ret.is_ok() == (value <= Self::MAX_SPACES))]
    #[ensures(ret.as_ref().is_ok_and(|width| width.spaces() == value) || ret.is_err())]
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value > Self::MAX_SPACES {
            return Err(InvalidIndentation);
        }
        Ok(new!(Indentation { spaces: value }))
    }
}

impl FromStr for Indentation {
    type Err = InvalidIndentation;

    #[requires(true)]
    #[ensures(true)]
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(text.parse::<usize>().map_err(|_| InvalidIndentation)?)
    }
}

impl fmt::Display for Indentation {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.spaces)
    }
}

impl<'de> Deserialize<'de> for Indentation {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_from(usize::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JsonRenderOptions, TreeRenderOptions, render_json_value_with_options};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn indentation_accepts_exactly_zero_through_eight() {
        for spaces in 0..=Indentation::MAX_SPACES {
            let width = Indentation::try_from(spaces).expect("accepted width");
            assert_eq!(width.spaces(), spaces);
            let json = serde_json::to_string(&width).expect("serialize width");
            assert_eq!(json, spaces.to_string());
            assert_eq!(serde_json::from_str::<Indentation>(&json).unwrap(), width);
            assert_eq!(json.parse::<Indentation>().unwrap(), width);
        }
        for spaces in [9, 256, 1_073_741_824, usize::MAX] {
            assert!(Indentation::try_from(spaces).is_err());
            let error = serde_json::from_str::<Indentation>(&spaces.to_string()).unwrap_err();
            assert!(error.to_string().contains("between 0 and 8"), "{error}");
        }
        for text in ["-1", "1.5", "18446744073709551616", "abc"] {
            assert!(text.parse::<Indentation>().is_err(), "{text}");
            assert!(serde_json::from_str::<Indentation>(text).is_err(), "{text}");
        }
        assert_eq!(Indentation::default().spaces(), 2);
        assert_eq!(Indentation::compact().spaces(), 0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn indentation_is_validated_in_serialized_renderer_configuration() {
        let mut json = serde_json::to_value(JsonRenderOptions::default()).unwrap();
        let mut tree = serde_json::to_value(TreeRenderOptions::default()).unwrap();
        json["indent"] = 9.into();
        tree["indent"] = 9.into();
        let json_error = serde_json::from_value::<JsonRenderOptions>(json).unwrap_err();
        let tree_error = serde_json::from_value::<TreeRenderOptions>(tree).unwrap_err();
        assert!(json_error.to_string().contains("between 0 and 8"));
        assert!(tree_error.to_string().contains("between 0 and 8"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn indentation_json_output_preserves_every_accepted_width() {
        let value = serde_json::json!({"entry": {"word": "mi"}});
        for spaces in 0..=Indentation::MAX_SPACES {
            let text = render_json_value_with_options(
                &value,
                JsonRenderOptions {
                    indent: Indentation::try_from(spaces).unwrap(),
                    ..JsonRenderOptions::default()
                },
            );
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&text).unwrap(),
                value
            );
            if spaces == 0 {
                assert!(!text.contains('\n'));
            } else {
                assert!(text.contains(&format!("\n{}\"entry\"", " ".repeat(spaces))));
                assert!(text.contains(&format!("\n{}\"word\"", " ".repeat(spaces * 2))));
            }
        }
    }
}

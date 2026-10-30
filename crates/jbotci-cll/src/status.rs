//! Structured authority notes from the book's `condition` attributes.

use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde::{Deserialize, Serialize};

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllRuleStatus {
    LlgRatified,
    BpfkApproved,
    Checkpointed,
    DeFacto,
    Proposed,
    Unsettled,
    Editorial,
    Historical,
}

impl CllRuleStatus {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "llg-ratified" => Ok(Self::LlgRatified),
            "bpfk-approved" => Ok(Self::BpfkApproved),
            "checkpointed" => Ok(Self::Checkpointed),
            "de-facto" => Ok(Self::DeFacto),
            "proposed" => Ok(Self::Proposed),
            "unsettled" => Ok(Self::Unsettled),
            "editorial" => Ok(Self::Editorial),
            "historical" => Ok(Self::Historical),
            _ => Err(format!("unknown rule status `{text}`")),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllStatusBody {
    Llg,
    Bpfk,
    Community,
}

#[invariant((1..=9999).contains(year))]
#[invariant(month.is_none_or(|month| (1..=12).contains(&month)))]
#[invariant(day.is_none() || month.is_some())]
#[invariant(day.is_none_or(|day| {
    let last_day = match month.unwrap_or(1) {
        2 if *year % 4 == 0 && (*year % 100 != 0 || *year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=last_day).contains(&day)
}))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllStatusDate {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

impl CllStatusDate {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
    fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text.split('-');
        let year_part = parts.next().unwrap_or_default();
        let month_part = parts.next();
        let day_part = parts.next();
        if parts.next().is_some()
            || year_part.len() != 4
            || !year_part.bytes().all(|byte| byte.is_ascii_digit())
            || [month_part, day_part]
                .into_iter()
                .flatten()
                .any(|part| part.len() != 2 || !part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(format!("invalid status date `{text}`"));
        }
        let year = year_part
            .parse::<u16>()
            .map_err(|error| error.to_string())?;
        let month = month_part
            .map(str::parse::<u8>)
            .transpose()
            .map_err(|error| error.to_string())?;
        let day = day_part
            .map(str::parse::<u8>)
            .transpose()
            .map_err(|error| error.to_string())?;
        bityzba::try_new!(CllStatusDate { year, month, day })
            .map_err(|_| format!("invalid calendar date `{text}`"))
    }
}

#[invariant(source.as_ref().is_none_or(|slug| {
    slug.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        && slug.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllStatusCondition {
    pub status: CllRuleStatus,
    pub also: Vec<CllRuleStatus>,
    pub date: Option<CllStatusDate>,
    pub body: Option<CllStatusBody>,
    pub source: Option<String>,
}

impl CllStatusCondition {
    /// Reads the microsyntax in `vendor/cll/docs/status-markup.md`.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut status = None;
        let mut also = Vec::new();
        let mut date = None;
        let mut body = None;
        let mut source = None;
        let mut keys = BTreeSet::new();
        for component in text.split(';') {
            let (key, value) = component
                .split_once(':')
                .ok_or_else(|| format!("invalid status component `{component}`"))?;
            let (key, value) = (key.trim(), value.trim());
            if key.is_empty() || value.is_empty() || !keys.insert(key) {
                return Err(format!("empty or duplicate status key `{key}`"));
            }
            match key {
                "status" => status = Some(CllRuleStatus::parse(value)?),
                "also" => {
                    also = value
                        .split(',')
                        .map(|token| CllRuleStatus::parse(token.trim()))
                        .collect::<Result<_, _>>()?
                }
                "date" => date = Some(CllStatusDate::parse(value)?),
                "body" => {
                    body = Some(match value {
                        "llg" => CllStatusBody::Llg,
                        "bpfk" => CllStatusBody::Bpfk,
                        "community" => CllStatusBody::Community,
                        _ => return Err(format!("unknown status body `{value}`")),
                    })
                }
                "src" => source = Some(value.to_owned()),
                // The upstream format reserves unknown keys for later extensions.
                _ => {}
            }
        }
        let status = status.ok_or_else(|| "status condition has no primary status".to_owned())?;
        bityzba::try_new!(CllStatusCondition {
            status,
            also,
            date,
            body,
            source
        })
        .map_err(|_| "invalid status source slug".to_owned())
    }

    #[requires(true)]
    #[ensures(self.body.is_none() || ret == self.body)]
    pub fn governing_body(&self) -> Option<CllStatusBody> {
        self.body.or(match self.status {
            CllRuleStatus::LlgRatified => Some(CllStatusBody::Llg),
            CllRuleStatus::BpfkApproved | CllRuleStatus::Checkpointed => Some(CllStatusBody::Bpfk),
            CllRuleStatus::DeFacto | CllRuleStatus::Proposed | CllRuleStatus::Unsettled => {
                Some(CllStatusBody::Community)
            }
            CllRuleStatus::Editorial | CllRuleStatus::Historical => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn status_conditions_preserve_authority_and_calendar_dates() {
        let condition = CllStatusCondition::parse("status:bpfk-approved;date:2016-03-15;also:historical,editorial;src:tag-binding;future:value").unwrap();
        assert_eq!(condition.status, CllRuleStatus::BpfkApproved);
        assert_eq!(condition.governing_body(), Some(CllStatusBody::Bpfk));
        assert_eq!(condition.date.as_ref().unwrap().day, Some(15));
        assert_eq!(
            condition.also,
            [CllRuleStatus::Historical, CllRuleStatus::Editorial]
        );
        assert_eq!(condition.source.as_deref(), Some("tag-binding"));
        assert_eq!(
            CllStatusCondition::parse("status:editorial")
                .unwrap()
                .governing_body(),
            None
        );
        for date in ["2000", "2000-02", "2000-02-29"] {
            assert!(CllStatusDate::parse(date).is_ok(), "{date}");
        }
        for date in [
            "0000",
            "1900-02-29",
            "2000-13",
            "2000-04-31",
            "2000-2",
            "2000-01-00",
        ] {
            assert!(CllStatusDate::parse(date).is_err(), "{date}");
        }
        for condition in [
            "",
            "status:editorial;",
            "status:editorial;;date:2000",
            "status:editorial;status:historical",
            "status:unknown",
            "date:2000",
            "status:editorial;also:",
            "status:editorial;body:owner",
            "status:editorial;src:-bad",
            "status:editorial;src:Bad",
        ] {
            assert!(CllStatusCondition::parse(condition).is_err(), "{condition}");
        }
    }
}

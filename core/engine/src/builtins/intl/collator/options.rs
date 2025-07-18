use std::str::FromStr;

use icu_collator::{
    options::{CaseLevel, Strength},
    preferences::CollationCaseFirst,
};

use crate::{
    Context, JsNativeError, JsResult, JsValue,
    builtins::options::{OptionType, ParsableOptionType},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Sensitivity {
    Base,
    Accent,
    Case,
    Variant,
}

impl Sensitivity {
    /// Converts the sensitivity option to the equivalent ICU4X collator options.
    pub(crate) const fn to_collator_options(self) -> (Strength, CaseLevel) {
        match self {
            Self::Base => (Strength::Primary, CaseLevel::Off),
            Self::Accent => (Strength::Secondary, CaseLevel::Off),
            Self::Case => (Strength::Primary, CaseLevel::On),
            Self::Variant => (Strength::Tertiary, CaseLevel::On),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ParseSensitivityError;

impl std::fmt::Display for ParseSensitivityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("provided string was not `base`, `accent`, `case` or `variant`")
    }
}

impl FromStr for Sensitivity {
    type Err = ParseSensitivityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "base" => Ok(Self::Base),
            "accent" => Ok(Self::Accent),
            "case" => Ok(Self::Case),
            "variant" => Ok(Self::Variant),
            _ => Err(ParseSensitivityError),
        }
    }
}

impl ParsableOptionType for Sensitivity {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Usage {
    #[default]
    Sort,
    Search,
}

#[derive(Debug)]
pub(crate) struct ParseUsageError;

impl std::fmt::Display for ParseUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("provided string was not `sort` or `search`")
    }
}

impl FromStr for Usage {
    type Err = ParseUsageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sort" => Ok(Self::Sort),
            "search" => Ok(Self::Search),
            _ => Err(ParseUsageError),
        }
    }
}

impl ParsableOptionType for Usage {}

impl OptionType for CollationCaseFirst {
    fn from_value(value: JsValue, context: &mut Context) -> JsResult<Self> {
        match value.to_string(context)?.to_std_string_escaped().as_str() {
            "upper" => Ok(Self::Upper),
            "lower" => Ok(Self::Lower),
            "false" => Ok(Self::False),
            _ => Err(JsNativeError::range()
                .with_message("provided string was not `upper`, `lower` or `false`")
                .into()),
        }
    }
}

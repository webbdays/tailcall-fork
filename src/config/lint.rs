use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, schemars::JsonSchema)]
pub enum TextCase {
    Camel,
    Pascal,
    Snake,
    ScreamingSnake,
}

impl Display for TextCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TextCase::Camel => "camelCase",
            TextCase::Pascal => "PascalCase",
            TextCase::Snake => "snake_case",
            TextCase::ScreamingSnake => "SCREAMING_SNAKE_CASE",
        })
    }
}

/// The @lint directive allows you to configure linting.
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Debug, Clone, schemars::JsonSchema)]
pub struct Lint {
    ///
    /// To autoFix the lint.
    /// Example Usage lint:{autoFix:true}, this is optional setting.
    #[serde(rename = "autoFix")]
    pub auto_fix: Option<bool>,
    ///
    ///
    /// We can specify the text case for the enum names.
    /// Example Usage: lint:{enum:Pascal}, this is optional setting.
    #[serde(rename = "enum")]
    pub enum_lint: Option<TextCase>,
    ///
    ///
    /// We can specify the text case for the values in enums.
    /// Example Usage: lint:{enumValue:ScreamingSnake}, this is optional setting.
    #[serde(rename = "enumValue")]
    pub enum_value_lint: Option<TextCase>,
    ///
    ///
    /// We can specify the text case for field names.
    /// Example Usage: lint:{field:Camel}, this is optional setting.
    #[serde(rename = "field")]
    pub field_lint: Option<TextCase>,
    ///
    ///
    /// We can specify the text case for type names.
    /// Example Usage: lint:{type:Pascal} , this is optional setting.
    #[serde(rename = "type")]
    pub type_lint: Option<TextCase>,
}

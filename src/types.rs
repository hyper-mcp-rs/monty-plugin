use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MONTY_OBJECT_SCHEMA_JSON: &str = include_str!("monty_object.schema.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginMontyObject(pub monty::MontyObject);

impl From<monty::MontyObject> for PluginMontyObject {
    fn from(value: monty::MontyObject) -> Self {
        Self(value)
    }
}

impl From<PluginMontyObject> for monty::MontyObject {
    fn from(value: PluginMontyObject) -> Self {
        value.0
    }
}

impl AsRef<monty::MontyObject> for PluginMontyObject {
    fn as_ref(&self) -> &monty::MontyObject {
        &self.0
    }
}

impl std::ops::Deref for PluginMontyObject {
    type Target = monty::MontyObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PluginMontyObject {
    pub fn into_inner(self) -> monty::MontyObject {
        self.0
    }
}

impl JsonSchema for PluginMontyObject {
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        serde_json::from_str::<Schema>(MONTY_OBJECT_SCHEMA_JSON)
            .expect("Invalid MontyObject schema")
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "MontyObject").into()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "MontyObject".into()
    }
}

/// Input for the `run` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunArguments {
    /// The Python code to execute.
    pub code: String,

    /// Input variables as a map of name to value.
    #[serde(default)]
    pub inputs: HashMap<String, PluginMontyObject>,
}

/// Output returned by the `run` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunResponse {
    /// Captured print/stdout output produced by the Python code.
    pub output: String,

    /// The return value of the Python code (serialized MontyObject).
    pub result: PluginMontyObject,
}

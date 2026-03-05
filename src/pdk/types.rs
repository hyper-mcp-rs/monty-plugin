#![allow(unused)]
use base64::engine::general_purpose::STANDARD;
use base64_serde::base64_serde_type;
use extism_pdk::{FromBytes, Json, ToBytes};
use oauth2::{ClientId, ClientSecret, DeviceAuthorizationUrl, Scope, TokenUrl};
use schemars::Schema as JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::{collections::HashMap, time::SystemTime};

base64_serde_type!(Base64Standard, STANDARD);

/// Validates that a type discriminator matches the expected value.
fn validate_type_field<E>(actual: &str, expected: &str, type_name: &str) -> Result<(), E>
where
    E: serde::de::Error,
{
    if actual != expected {
        Err(E::custom(format!(
            "invalid type for {type_name}: expected '{expected}', found '{actual}'"
        )))
    } else {
        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct Annotations {
    /// Intended audience for the resource
    pub audience: Vec<Role>,

    /// Last modified timestamp for the resource
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,

    /// Priority level indicating the importance of the resource
    pub priority: f32,
}

/// An access token returned by the host's OAuth2 token management.
///
/// The host handles token acquisition, caching, and refresh transparently.
/// Plugins receive a ready-to-use bearer token.
#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct AccessToken {
    /// The bearer token.
    pub access_token: oauth2::AccessToken,
    /// When the token expires (if known).
    pub expires_at: Option<SystemTime>,
    /// The scopes granted by the token (if known).
    pub scopes: Option<Vec<Scope>>,
}

/// The type of client authentication to use when talking to the token endpoint.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthType {
    RequestBody,
    BasicAuth,
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct AudioContent {
    /// Optional additional metadata about the content block
    pub meta: Option<Meta>,

    /// Optional content annotations
    pub annotations: Option<Annotations>,

    /// Base64-encoded audio data
    pub data: String,

    /// MIME type of the audio (e.g. 'audio/mpeg')
    pub mime_type: String,
}

impl Serialize for AudioContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct AudioContentHelper<'a> {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: &'a Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            annotations: &'a Option<Annotations>,
            data: &'a String,
            #[serde(rename = "mimeType")]
            mime_type: &'a String,
            r#type: &'static str,
        }

        let helper = AudioContentHelper {
            meta: &self.meta,
            annotations: &self.annotations,
            data: &self.data,
            mime_type: &self.mime_type,
            r#type: "audio",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AudioContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AudioContentHelper {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            meta: Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            annotations: Option<Annotations>,
            data: String,
            #[serde(rename = "mimeType")]
            mime_type: String,
            r#type: String,
        }

        let helper = AudioContentHelper::deserialize(deserializer)?;
        validate_type_field(&helper.r#type, "audio", "AudioContent")?;

        Ok(AudioContent {
            meta: helper.meta,
            annotations: helper.annotations,
            data: helper.data,
            mime_type: helper.mime_type,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct BlobResourceContents {
    /// Optional additional metadata about the blob resource
    #[serde(rename = "_meta")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub meta: Option<Meta>,

    /// Base64-encoded binary data of the resource
    pub blob: String,

    /// MIME type of the binary content (e.g. 'application/pdf')
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mime_type: Option<String>,

    /// URI of the resource
    pub uri: String,
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct BooleanSchema {
    /// Optional default value
    pub default: Option<bool>,

    /// Description of the boolean input
    pub description: Option<String>,

    /// Optional human-readable title
    pub title: Option<String>,
}

impl Serialize for BooleanSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct BooleanSchemaHelper<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            default: &'a Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: &'a Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: &'a Option<String>,
            r#type: &'static str,
        }

        let helper = BooleanSchemaHelper {
            default: &self.default,
            description: &self.description,
            title: &self.title,
            r#type: "boolean",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BooleanSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BooleanSchemaHelper {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            default: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            title: Option<String>,
            r#type: String,
        }

        let helper = BooleanSchemaHelper::deserialize(deserializer)?;

        if helper.r#type != "boolean" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for BooleanSchema: expected 'boolean', found '{}'",
                helper.r#type
            )));
        }

        Ok(BooleanSchema {
            default: helper.default,
            description: helper.description,
            title: helper.title,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CallToolRequest {
    pub context: PluginRequestContext,

    pub request: CallToolRequestParam,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CallToolRequestParam {
    /// Arguments to pass to the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub arguments: Option<Map<String, Value>>,

    /// The name of the tool to call
    pub name: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CallToolResult {
    /// Optional additional metadata about the tool call result
    #[serde(rename = "_meta")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub meta: Option<Meta>,

    /// Array of TextContent, ImageContent, AudioContent, EmbeddedResource, or ResourceLinks representing the result
    pub content: Vec<ContentBlock>,

    /// Whether the tool call ended in an error. If not set, defaults to false.
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub is_error: Option<bool>,

    /// Optional structured JSON result from the tool
    #[serde(rename = "structuredContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub structured_content: Option<Map<String, Value>>,
}

impl CallToolResult {
    /// Creates an error `CallToolResult` with the given message.
    pub fn error(error: String) -> CallToolResult {
        CallToolResult {
            is_error: Some(true),
            content: vec![ContentBlock::Text(TextContent {
                text: error,
                ..Default::default()
            })],
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CompleteRequest {
    pub context: PluginRequestContext,

    pub request: CompleteRequestParam,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CompleteRequestParam {
    pub argument: CompleteRequestParamArgument,

    /// Optional completion context with previously-resolved arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompleteRequestParamContext>,

    /// Reference to either a PromptReference or ResourceTemplateReference
    pub r#ref: Reference,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CompleteRequestParamArgument {
    /// Name of the argument
    pub name: String,

    /// Current value to complete
    pub value: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CompleteRequestParamContext {
    /// Previously-resolved argument values
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub arguments: Option<HashMap<String, String>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CompleteResult {
    pub completion: CompleteResultCompletion,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CompleteResultCompletion {
    /// Whether there are more completions available
    #[serde(rename = "hasMore")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub has_more: Option<bool>,

    /// Total number of available completions
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub total: Option<i64>,

    /// Array of completion values (max 100 items)
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum ContentBlock {
    Audio(AudioContent),
    EmbeddedResource(EmbeddedResource),
    Image(ImageContent),
    ResourceLink(ResourceLink),
    Text(TextContent),
    Empty(Empty),
}

impl Default for ContentBlock {
    fn default() -> Self {
        ContentBlock::Empty(Empty::default())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CreateMessageRequestParam {
    #[serde(rename = "includeContext")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub include_context: Option<CreateMessageRequestParamIncludeContext>,

    /// Maximum tokens to sample
    #[serde(rename = "maxTokens")]
    pub max_tokens: i64,

    /// Conversation messages of of TextContent, ImageContent or AudioContent
    pub messages: Vec<SamplingMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub metadata: Option<Value>,

    /// Preferences for model selection
    #[serde(rename = "modelPreferences")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub model_preferences: Option<ModelPreferences>,

    /// Stop sequences
    #[serde(rename = "stopSequences")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub stop_sequences: Option<Vec<String>>,

    /// Optional system prompt
    #[serde(rename = "systemPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub system_prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub task: Option<Map<String, Value>>,

    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub temperature: Option<f64>,

    #[serde(rename = "toolChoice")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub tool_choice: Option<ToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum CreateMessageRequestParamIncludeContext {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "thisServer")]
    ThisServer,
    #[serde(rename = "allServers")]
    AllServers,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct CreateMessageResult {
    /// One of TextContent, ImageContent or AudioContent
    pub content: CreateMessageResultContent,

    /// Name of the model used
    pub model: String,

    pub role: Role,

    /// Optional reason sampling stopped
    #[serde(rename = "stopReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub stop_reason: Option<String>,
}

type CreateMessageResultContent = SamplingMessage;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ElicitationResponseNotificationParam {
    /// URI of the updated resource
    #[serde(rename = "elicitationId")]
    pub elicitation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(tag = "mode")]
pub enum ElicitationRequestParam {
    #[serde(rename = "form")]
    Form {
        /// Message to present to the user
        #[serde(rename = "message")]
        message: String,

        #[serde(rename = "requestedSchema")]
        requested_schema: Schema,
    },
    #[serde(rename = "url")]
    Url {
        #[serde(rename = "elicitationId")]
        elicitation_id: String,

        /// Message to present to the user
        #[serde(rename = "message")]
        message: String,

        #[serde(rename = "url")]
        url: String,
    },
}

impl Default for ElicitationRequestParam {
    fn default() -> Self {
        ElicitationRequestParam::Form {
            message: String::new(),
            requested_schema: Schema::default(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ElicitationRequestParamWithTimeout {
    #[serde(flatten)]
    pub inner: ElicitationRequestParam,

    /// Optional timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub timeout: Option<i64>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ElicitationResult {
    pub action: ElicitationResultAction,

    /// Form data submitted by user (only present when action is accept)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content: Option<HashMap<String, ElicitationResultContentValue>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum ElicitationResultAction {
    #[default]
    #[serde(rename = "accept")]
    Accept,
    #[serde(rename = "decline")]
    Decline,
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum ElicitationResultContentValue {
    String(String),
    Number(Number), // or serde_json::Number if you want exactness
    Bool(bool),
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct EmbeddedResource {
    /// Optional additional metadata about the embedded resource
    pub meta: Option<Meta>,

    /// Optional resource annotations
    pub annotations: Option<Annotations>,

    /// The embedded TextResourceContents or BlobResourceContents
    pub resource: ResourceContents,
}

impl Serialize for EmbeddedResource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct EmbeddedResourceHelper<'a> {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: &'a Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            annotations: &'a Option<Annotations>,
            resource: &'a ResourceContents,
            r#type: &'static str,
        }

        let helper = EmbeddedResourceHelper {
            meta: &self.meta,
            annotations: &self.annotations,
            resource: &self.resource,
            r#type: "resource",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EmbeddedResource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EmbeddedResourceHelper {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            meta: Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            annotations: Option<Annotations>,
            resource: ResourceContents,
            r#type: String,
        }

        let helper = EmbeddedResourceHelper::deserialize(deserializer)?;

        if helper.r#type != "resource" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for EmbeddedResource: expected 'resource', found '{}'",
                helper.r#type
            )));
        }

        Ok(EmbeddedResource {
            meta: helper.meta,
            annotations: helper.annotations,
            resource: helper.resource,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct EnumSchema {
    /// Description of the enum input
    pub description: Option<String>,

    /// Array of allowed string values
    pub r#enum: Vec<String>,

    /// Optional array of human-readable names for the enum values
    pub enum_names: Option<Vec<String>>,

    /// Optional human-readable title
    pub title: Option<String>,
}

impl Serialize for EnumSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct EnumSchemaHelper<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            description: &'a Option<String>,
            r#enum: &'a Vec<String>,
            #[serde(rename = "enumNames")]
            #[serde(skip_serializing_if = "Option::is_none")]
            enum_names: &'a Option<Vec<String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: &'a Option<String>,
            r#type: &'static str,
        }

        let helper = EnumSchemaHelper {
            description: &self.description,
            r#enum: &self.r#enum,
            enum_names: &self.enum_names,
            title: &self.title,
            r#type: "string",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EnumSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EnumSchemaHelper {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            description: Option<String>,
            r#enum: Vec<String>,
            #[serde(rename = "enumNames")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            enum_names: Option<Vec<String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            title: Option<String>,
            r#type: String,
        }

        let helper = EnumSchemaHelper::deserialize(deserializer)?;

        if helper.r#type != "string" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for EnumSchema: expected 'string', found '{}'",
                helper.r#type
            )));
        }

        Ok(EnumSchema {
            description: helper.description,
            r#enum: helper.r#enum,
            enum_names: helper.enum_names,
            title: helper.title,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct GetPromptRequest {
    pub context: PluginRequestContext,

    pub request: GetPromptRequestParam,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct GetPromptRequestParam {
    /// Arguments for templating the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub arguments: Option<HashMap<String, String>>,

    /// Name of the prompt to retrieve
    pub name: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct GetPromptResult {
    /// Optional description of the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// Array of prompt messages
    pub messages: Vec<PromptMessage>,
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ImageContent {
    /// Optional additional metadata about the content block
    pub meta: Option<Meta>,

    /// Optional content annotations
    pub annotations: Option<Annotations>,

    /// Base64-encoded image data
    pub data: String,

    /// MIME type of the image (e.g. 'image/png')
    pub mime_type: String,
}

impl Serialize for ImageContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ImageContentHelper<'a> {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: &'a Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            annotations: &'a Option<Annotations>,
            data: &'a String,
            #[serde(rename = "mimeType")]
            mime_type: &'a String,
            r#type: &'static str,
        }

        let helper = ImageContentHelper {
            meta: &self.meta,
            annotations: &self.annotations,
            data: &self.data,
            mime_type: &self.mime_type,
            r#type: "image",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ImageContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ImageContentHelper {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            meta: Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            annotations: Option<Annotations>,
            data: String,
            #[serde(rename = "mimeType")]
            mime_type: String,
            r#type: String,
        }

        let helper = ImageContentHelper::deserialize(deserializer)?;

        if helper.r#type != "image" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for ImageContent: expected 'image', found '{}'",
                helper.r#type
            )));
        }

        Ok(ImageContent {
            meta: helper.meta,
            annotations: helper.annotations,
            data: helper.data,
            mime_type: helper.mime_type,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct KeyringEntryId {
    pub service: String,
    pub user: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListPromptsRequest {
    pub context: PluginRequestContext,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListPromptsResult {
    /// Array of available prompts
    pub prompts: Vec<Prompt>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListResourcesRequest {
    pub context: PluginRequestContext,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListResourcesResult {
    /// Array of available resources
    pub resources: Vec<Resource>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListResourceTemplatesRequest {
    pub context: PluginRequestContext,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListResourceTemplatesResult {
    /// Array of resource templates
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: Vec<ResourceTemplate>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListRootsResult {
    /// Array of root directories/resources
    pub roots: Vec<Root>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListToolsRequest {
    pub context: PluginRequestContext,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ListToolsResult {
    /// Array of available tools
    pub tools: Vec<Tool>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum LoggingLevel {
    #[default]
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "notice")]
    Notice,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "emergency")]
    Emergency,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct LoggingMessageNotificationParam {
    /// Data to log (any JSON-serializable type)
    pub data: Value,

    pub level: LoggingLevel,

    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub logger: Option<String>,
}

type Meta = Map<String, Value>;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ModelHint {
    /// Suggested model name or family
    pub name: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ModelPreferences {
    /// Priority for cost (0-1)
    pub cost_priority: f32,

    /// Model name hints
    pub hints: Vec<ModelHint>,

    /// Priority for intelligence (0-1)
    #[serde(rename = "intelligencePriority")]
    pub intelligence_priority: f32,

    /// Priority for speed (0-1)
    #[serde(rename = "speedPriority")]
    pub speed_priority: f32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct NumberSchema {
    /// Description of the number input
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// Maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub maximum: Option<f64>,

    /// Minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub minimum: Option<f64>,

    /// Optional human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,

    pub r#type: NumberType,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum NumberType {
    #[default]
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "integer")]
    Integer,
}

/// Credentials needed to obtain an OAuth2 access token from the host.
///
/// Pass this to [`get_access_token`](super::imports::get_access_token) and
/// the host will return a cached or freshly-acquired [`AccessToken`].
#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct OauthCredentials {
    /// How to authenticate at the token endpoint.
    pub auth_type: Option<AuthType>,
    /// The OAuth2 client ID.
    pub client_id: ClientId,
    /// The OAuth2 client secret (if required).
    pub client_secret: Option<ClientSecret>,
    /// Device authorization endpoint URL. When present the host will use the
    /// device-code flow (with user elicitation) instead of client-credentials.
    pub device_authorization_url: Option<DeviceAuthorizationUrl>,
    /// Timeout in seconds for the device authorization flow. Defaults to 180 (3 minutes).
    pub device_auth_timeout_secs: Option<u64>,
    /// Extra parameters to send to the token endpoint.
    pub extra_params: Option<HashMap<String, String>>,
    /// Scopes to request.
    pub scopes: Option<Vec<Scope>>,
    /// The token endpoint URL.
    pub token_endpoint_url: TokenUrl,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct PluginNotificationContext {
    /// Additional metadata about the notification
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct PluginRequestContext {
    /// Additional metadata about the request
    #[serde(rename = "_meta")]
    pub meta: Meta,

    /// Unique identifier for this request
    pub id: PluginRequestId,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum PluginRequestId {
    String(String),
    Number(i64),
}

impl Default for PluginRequestId {
    fn default() -> Self {
        PluginRequestId::String(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum PrimitiveSchemaDefinition {
    Boolean(BooleanSchema),
    Enum(EnumSchema),
    Number(NumberSchema),
    String(StringSchema),
    Empty(Empty),
}

impl Default for PrimitiveSchemaDefinition {
    fn default() -> Self {
        PrimitiveSchemaDefinition::Empty(Empty::default())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ProgressNotificationParam {
    /// Optional progress message describing current operation
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub message: Option<String>,

    /// The progress thus far
    pub progress: f64,

    /// A token identifying the progress context
    #[serde(rename = "progressToken")]
    pub progress_token: String,

    /// Optional total units of work
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub total: Option<f64>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct Prompt {
    /// Optional prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub arguments: Option<Vec<PromptArgument>>,

    /// Description of what the prompt does
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// Unique name of the prompt
    pub name: String,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct PromptArgument {
    /// Description of the argument
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// Name of the argument
    pub name: String,

    /// Whether this argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub required: Option<bool>,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct PromptMessage {
    /// One of TextContent, ImageContent, AudioContent, EmbeddedResource, or ResourceLink
    pub content: ContentBlock,

    pub role: Role,
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct PromptReference {
    /// Name of the prompt
    pub name: String,

    /// Optional human-readable title
    pub title: Option<String>,
}

impl Serialize for PromptReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct PromptReferenceHelper<'a> {
            name: &'a String,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: &'a Option<String>,
            r#type: &'static str,
        }

        let helper = PromptReferenceHelper {
            name: &self.name,
            title: &self.title,
            r#type: "prompt",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PromptReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PromptReferenceHelper {
            name: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            title: Option<String>,
            r#type: String,
        }

        let helper = PromptReferenceHelper::deserialize(deserializer)?;

        if helper.r#type != "prompt" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for PromptReference: expected 'prompt', found '{}'",
                helper.r#type
            )));
        }

        Ok(PromptReference {
            name: helper.name,
            title: helper.title,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ReadResourceRequest {
    pub context: PluginRequestContext,

    pub request: ReadResourceRequestParam,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ReadResourceRequestParam {
    /// URI of the resource to read
    pub uri: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ReadResourceResult {
    /// Array of TextResourceContents or BlobResourceContents
    pub contents: Vec<ResourceContents>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum Reference {
    Prompt(PromptReference),
    ResourceTemplate(ResourceTemplateReference),
    Empty(Empty),
}

impl Default for Reference {
    fn default() -> Self {
        Reference::Empty(Empty::default())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct Resource {
    /// Optional resource annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub annotations: Option<Annotations>,

    /// Description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// MIME type of the resource
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mime_type: Option<String>,

    /// Human-readable name
    pub name: String,

    /// Size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub size: Option<i64>,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,

    /// URI of the resource
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum ResourceContents {
    Blob(BlobResourceContents),
    Text(TextResourceContents),
    Empty(Empty),
}

impl Default for ResourceContents {
    fn default() -> Self {
        ResourceContents::Empty(Empty::default())
    }
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ResourceLink {
    /// Optional additional metadata about the resource link
    pub meta: Option<Meta>,

    /// Optional resource annotations
    pub annotations: Option<Annotations>,

    /// Optional description of the resource
    pub description: Option<String>,

    /// Optional MIME type of the resource
    pub mime_type: Option<String>,

    /// Optional human-readable name
    pub name: String,

    /// Optional size in bytes
    pub size: Option<i64>,

    /// Optional human-readable title
    pub title: Option<String>,

    /// URI of the resource
    pub uri: String,
}

impl Serialize for ResourceLink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ResourceLinkHelper<'a> {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: &'a Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            annotations: &'a Option<Annotations>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: &'a Option<String>,
            #[serde(rename = "mimeType")]
            #[serde(skip_serializing_if = "Option::is_none")]
            mime_type: &'a Option<String>,
            name: &'a String,
            #[serde(skip_serializing_if = "Option::is_none")]
            size: &'a Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: &'a Option<String>,
            r#type: &'static str,
            uri: &'a String,
        }

        let helper = ResourceLinkHelper {
            meta: &self.meta,
            annotations: &self.annotations,
            description: &self.description,
            mime_type: &self.mime_type,
            name: &self.name,
            size: &self.size,
            title: &self.title,
            r#type: "resource_link",
            uri: &self.uri,
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ResourceLink {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ResourceLinkHelper {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            meta: Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            annotations: Option<Annotations>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            description: Option<String>,
            #[serde(rename = "mimeType")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            mime_type: Option<String>,
            name: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            size: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            title: Option<String>,
            r#type: String,
            uri: String,
        }

        let helper = ResourceLinkHelper::deserialize(deserializer)?;

        if helper.r#type != "resource_link" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for ResourceLink: expected 'resource_link', found '{}'",
                helper.r#type
            )));
        }

        Ok(ResourceLink {
            meta: helper.meta,
            annotations: helper.annotations,
            description: helper.description,
            mime_type: helper.mime_type,
            name: helper.name,
            size: helper.size,
            title: helper.title,
            uri: helper.uri,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ResourceTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub annotations: Option<Annotations>,

    /// Description of the template
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// MIME type for resources matching this template
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mime_type: Option<String>,

    /// Human-readable name
    pub name: String,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,

    /// RFC 6570 URI template pattern
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ResourceTemplateReference {
    /// URI or URI template pattern of the resource
    pub uri: String,
}

impl Serialize for ResourceTemplateReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ResourceTemplateReferenceHelper<'a> {
            r#type: &'static str,
            uri: &'a String,
        }

        let helper = ResourceTemplateReferenceHelper {
            r#type: "resource",
            uri: &self.uri,
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ResourceTemplateReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ResourceTemplateReferenceHelper {
            r#type: String,
            uri: String,
        }

        let helper = ResourceTemplateReferenceHelper::deserialize(deserializer)?;

        if helper.r#type != "resource" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for ResourceTemplateReference: expected 'resource', found '{}'",
                helper.r#type
            )));
        }

        Ok(ResourceTemplateReference { uri: helper.uri })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ResourceUpdatedNotificationParam {
    /// URI of the updated resource
    pub uri: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum Role {
    #[default]
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "user")]
    User,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct Root {
    /// Optional human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,

    /// URI of the root (typically file://)
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
#[serde(untagged)]
pub enum SamplingMessage {
    Audio(AudioContent),
    Image(ImageContent),
    Text(TextContent),
    Empty(Empty),
}

impl Default for SamplingMessage {
    fn default() -> Self {
        SamplingMessage::Empty(Empty::default())
    }
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct Schema {
    /// A map of StringSchema, NumberSchema, BooleanSchema or EnumSchema definitions (no nesting)
    pub properties: HashMap<String, PrimitiveSchemaDefinition>,

    /// Required property names
    pub required: Option<Vec<String>>,
}

impl Serialize for Schema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct SchemaHelper<'a> {
            properties: &'a HashMap<String, PrimitiveSchemaDefinition>,
            #[serde(skip_serializing_if = "Option::is_none")]
            required: &'a Option<Vec<String>>,
            r#type: &'static str,
        }

        let helper = SchemaHelper {
            properties: &self.properties,
            required: &self.required,
            r#type: "object",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SchemaHelper {
            properties: HashMap<String, PrimitiveSchemaDefinition>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            required: Option<Vec<String>>,
            r#type: String,
        }

        let helper = SchemaHelper::deserialize(deserializer)?;

        if helper.r#type != "object" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for Schema: expected 'object', found '{}'",
                helper.r#type
            )));
        }

        Ok(Schema {
            properties: helper.properties,
            required: helper.required,
        })
    }
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct StringSchema {
    /// Description of the string input
    pub description: Option<String>,

    pub format: Option<StringSchemaFormat>,

    /// Maximum length of the string
    pub max_length: Option<i64>,

    /// Minimum length of the string
    pub min_length: Option<i64>,

    /// Optional human-readable title
    pub title: Option<String>,
}

impl Serialize for StringSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct StringSchemaHelper<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            description: &'a Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            format: &'a Option<StringSchemaFormat>,
            #[serde(rename = "maxLength")]
            #[serde(skip_serializing_if = "Option::is_none")]
            max_length: &'a Option<i64>,
            #[serde(rename = "minLength")]
            #[serde(skip_serializing_if = "Option::is_none")]
            min_length: &'a Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: &'a Option<String>,
            r#type: &'static str,
        }

        let helper = StringSchemaHelper {
            description: &self.description,
            format: &self.format,
            max_length: &self.max_length,
            min_length: &self.min_length,
            title: &self.title,
            r#type: "string",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StringSchemaHelper {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            format: Option<StringSchemaFormat>,
            #[serde(rename = "maxLength")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            max_length: Option<i64>,
            #[serde(rename = "minLength")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            min_length: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            title: Option<String>,
            r#type: String,
        }

        let helper = StringSchemaHelper::deserialize(deserializer)?;

        if helper.r#type != "string" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for StringSchema: expected 'string', found '{}'",
                helper.r#type
            )));
        }

        Ok(StringSchema {
            description: helper.description,
            format: helper.format,
            max_length: helper.max_length,
            min_length: helper.min_length,
            title: helper.title,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum StringSchemaFormat {
    #[default]
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "uri")]
    Uri,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "date_time")]
    Datetime,
}

#[derive(Default, Debug, Clone, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct TextContent {
    /// Optional additional metadata about the content block
    pub meta: Option<Meta>,

    /// Optional content annotations
    pub annotations: Option<Annotations>,

    /// The text content
    pub text: String,
}

impl Serialize for TextContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct TextContentHelper<'a> {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: &'a Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            annotations: &'a Option<Annotations>,
            text: &'a String,
            r#type: &'static str,
        }

        let helper = TextContentHelper {
            meta: &self.meta,
            annotations: &self.annotations,
            text: &self.text,
            r#type: "text",
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TextContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TextContentHelper {
            #[serde(rename = "_meta")]
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            meta: Option<Meta>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            annotations: Option<Annotations>,
            text: String,
            r#type: String,
        }

        let helper = TextContentHelper::deserialize(deserializer)?;

        if helper.r#type != "text" {
            return Err(serde::de::Error::custom(format!(
                "invalid type for TextContent: expected 'text', found '{}'",
                helper.r#type
            )));
        }

        Ok(TextContent {
            meta: helper.meta,
            annotations: helper.annotations,
            text: helper.text,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct TextResourceContents {
    /// Optional additional metadata about the text resource
    #[serde(rename = "_meta")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub meta: Option<Meta>,

    /// MIME type of the text content (e.g. 'text/plain')
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mime_type: Option<String>,

    /// Text content of the resource
    pub text: String,

    /// URI of the resource
    pub uri: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct Tool {
    /// Optional tool annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub annotations: Option<ToolAnnotations>,

    /// Description of what the tool does
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    #[serde(rename = "inputSchema")]
    pub input_schema: JsonSchema,

    /// Unique name of the tool
    pub name: String,

    #[serde(rename = "outputSchema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub output_schema: Option<JsonSchema>,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "destructiveHint")]
    pub destructive_hint: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "idempotentHint")]
    pub idempotent_hint: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "openWorldHint")]
    pub open_world_hint: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "readOnlyHint")]
    pub read_only_hint: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub struct ToolChoice {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mode: Option<ToolChoiceMode>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromBytes, ToBytes)]
#[encoding(Json)]
pub enum ToolChoiceMode {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "required")]
    Required,
    #[serde(rename = "none")]
    None,
}

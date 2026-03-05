mod pdk;

use anyhow::{Result, anyhow};
use pdk::types::*;

pub(crate) fn call_tool(_input: CallToolRequest) -> Result<CallToolResult> {
    Err(anyhow!("call_tool not implemented"))
}

// Provide completion suggestions for a partially-typed input.
//
// This function is called when the user requests autocompletion. The plugin should analyze the partial input and return matching completion suggestions based on the reference (prompt or resource) and argument context.
pub(crate) fn complete(_input: CompleteRequest) -> Result<CompleteResult> {
    Ok(CompleteResult::default())
}

// Retrieve a specific prompt by name.
//
// This function is called when the user requests a specific prompt. The plugin should return the prompt details including messages and optional description.
pub(crate) fn get_prompt(_input: GetPromptRequest) -> Result<GetPromptResult> {
    Err(anyhow!("get_prompt not implemented"))
}

// List all available prompts.
//
// This function should return a list of prompts that the plugin provides. Each prompt should include its name and a brief description of what it does. Supports pagination via cursor.
pub(crate) fn list_prompts(_input: ListPromptsRequest) -> Result<ListPromptsResult> {
    Ok(ListPromptsResult::default())
}

// List all available resource templates.
//
// This function should return a list of resource templates that the plugin provides. Templates are URI patterns that can match multiple resources. Supports pagination via cursor.
pub(crate) fn list_resource_templates(
    _input: ListResourceTemplatesRequest,
) -> Result<ListResourceTemplatesResult> {
    Ok(ListResourceTemplatesResult::default())
}

// List all available resources.
//
// This function should return a list of resources that the plugin provides. Resources are URI-based references to files, data, or services. Supports pagination via cursor.
pub(crate) fn list_resources(_input: ListResourcesRequest) -> Result<ListResourcesResult> {
    Ok(ListResourcesResult::default())
}

// List all available tools.
//
// This function should return a list of all tools that the plugin provides. Each tool should include its name, description, and input schema. Supports pagination via cursor.
pub(crate) fn list_tools(_input: ListToolsRequest) -> Result<ListToolsResult> {
    Ok(ListToolsResult::default())
}

// Notification that the list of roots has changed.
//
// This is an optional notification handler. If implemented, the plugin will be notified whenever the roots list changes on the client side. This allows plugins to react to changes in the file system roots or other root resources.
pub(crate) fn on_roots_list_changed(_input: PluginNotificationContext) -> Result<()> {
    Ok(())
}

// Read the contents of a resource by its URI.
//
// This function is called when the user wants to read the contents of a specific resource. The plugin should retrieve and return the resource data with appropriate MIME type information.
pub(crate) fn read_resource(_input: ReadResourceRequest) -> Result<ReadResourceResult> {
    Err(anyhow!("read_resource not implemented"))
}

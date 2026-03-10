mod function_calls;
mod monty;
mod os_calls;
mod pdk;
mod python_args;
mod types;

use crate::{
    monty::run_monty,
    os_calls::OS_CALLS_DESCRIPTION,
    pdk::types::*,
    types::{RunArguments, RunResponse},
};
use ::monty::MontyObject;
use anyhow::{Result, anyhow};
use function_calls::{BUILTIN_FUNCTIONS_DESCRIPTION, handle_function_call};
use os_calls::handle_os_call;
use serde_json::{Map, Value};

// =========================================================================
// MCP entry-points
// =========================================================================

pub(crate) fn call_tool(input: CallToolRequest) -> Result<CallToolResult> {
    if input.request.name != "run" {
        return Ok(CallToolResult::error(format!(
            "unknown tool: {}",
            input.request.name
        )));
    }

    let progress_token = input
        .context
        .meta
        .get("progressToken")
        .and_then(|v| serde_json::from_value::<ProgressToken>(v.clone()).ok());
    let progress_token_ref = progress_token.as_ref();

    // Deserialize arguments into our schemars-typed struct
    let args_map = input.request.arguments.unwrap_or_default();
    let args_value = Value::Object(args_map);
    let run_input: RunArguments = serde_json::from_value(args_value)
        .map_err(|e| anyhow!("invalid arguments for 'run' tool: {e}"))?;

    match run_monty(
        run_input,
        |name, args, kwargs| handle_function_call(name, args, kwargs, progress_token_ref),
        handle_os_call,
    ) {
        Ok(ref tool_output) => {
            let structured: Map<String, Value> = match serde_json::to_value(tool_output)? {
                Value::Object(m) => m,
                other => {
                    let mut m = Map::new();
                    m.insert("value".into(), other);
                    m
                }
            };

            // Build human-readable text content
            let mut text_parts: Vec<String> = Vec::new();
            if !tool_output.output.is_empty() {
                text_parts.push(format!("--- stdout ---\n{}", tool_output.output));
            }

            let is_error = match tool_output.result.as_ref() {
                MontyObject::Exception { exc_type, arg } => {
                    let detail = match arg {
                        Some(a) => format!("{exc_type}: {a}"),
                        None => format!("{exc_type}"),
                    };
                    text_parts.push(format!("--- exception ---\n{}", detail));
                    true
                }
                _ => {
                    text_parts.push(format!(
                        "--- result ---\n{}",
                        serde_json::to_string_pretty(&tool_output.result)?
                    ));
                    false
                }
            };

            Ok(CallToolResult {
                meta: None,
                content: vec![ContentBlock::Text(TextContent {
                    meta: None,
                    annotations: None,
                    text: text_parts.join("\n"),
                })],
                is_error: Some(is_error),
                structured_content: Some(structured),
            })
        }
        Err(e) => Ok(CallToolResult::error(format!("{e}"))),
    }
}

pub(crate) fn list_tools(_input: ListToolsRequest) -> Result<ListToolsResult> {
    Ok(ListToolsResult {
        tools: vec![Tool {
            annotations: None,
            description: Some(format!(
                "Execute Python code in a sandboxed Monty interpreter (https://github.com/pydantic/monty).\n\
                 \n\
                 {BUILTIN_FUNCTIONS_DESCRIPTION}\n\
                 \n\
                 {OS_CALLS_DESCRIPTION}"
            )),
            input_schema: schemars::schema_for!(RunArguments),
            name: "run".into(),
            output_schema: Some(schemars::schema_for!(RunResponse)),
            title: Some("Run Python Code".into()),
        }],
    })
}

// =========================================================================
// Other MCP handlers (unchanged stubs)
// =========================================================================

pub(crate) fn complete(_input: CompleteRequest) -> Result<CompleteResult> {
    Ok(CompleteResult::default())
}

pub(crate) fn get_prompt(_input: GetPromptRequest) -> Result<GetPromptResult> {
    Err(anyhow!("get_prompt not implemented"))
}

pub(crate) fn list_prompts(_input: ListPromptsRequest) -> Result<ListPromptsResult> {
    Ok(ListPromptsResult::default())
}

pub(crate) fn list_resource_templates(
    _input: ListResourceTemplatesRequest,
) -> Result<ListResourceTemplatesResult> {
    Ok(ListResourceTemplatesResult::default())
}

pub(crate) fn list_resources(_input: ListResourcesRequest) -> Result<ListResourcesResult> {
    Ok(ListResourcesResult::default())
}

pub(crate) fn on_roots_list_changed(_input: PluginNotificationContext) -> Result<()> {
    Ok(())
}

pub(crate) fn read_resource(_input: ReadResourceRequest) -> Result<ReadResourceResult> {
    Err(anyhow!("read_resource not implemented"))
}

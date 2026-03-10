mod function_calls;
mod os_calls;
mod pdk;
mod python_args;
mod types;

use anyhow::{Result, anyhow};
use function_calls::{BUILTIN_FUNCTIONS_DESCRIPTION, handle_function_call};
use monty::{ExtFunctionResult, LimitedTracker, MontyObject, MontyRun, PrintWriter, RunProgress};
use os_calls::{OS_CALLS_DESCRIPTION, handle_os_call};
use pdk::types::*;
use serde_json::{Map, Value};
use types::{PluginMontyObject, RunArguments, RunResponse};

// ---------------------------------------------------------------------------
// Run the Monty execution loop
// ---------------------------------------------------------------------------

fn run_monty(input: RunArguments, progress_token: Option<&ProgressToken>) -> Result<RunResponse> {
    let input_names: Vec<String> = input.inputs.keys().cloned().collect();

    let runner = MontyRun::new(input.code.clone(), "plugin.py", input_names.clone())
        .map_err(|e| anyhow!("failed to parse Python code: {e}"))?;

    // Convert inputs in the same order as input_names
    let monty_inputs: Vec<MontyObject> = input_names
        .iter()
        .map(|name| {
            input
                .inputs
                .get(name)
                .cloned()
                .unwrap_or(PluginMontyObject(MontyObject::None))
                .into_inner()
        })
        .collect();

    let mut output_buf = String::new();
    let mut writer = PrintWriter::Collect(&mut output_buf);

    let mut progress = runner
        .start(
            monty_inputs,
            LimitedTracker::new(input.resource_limits.unwrap_or_default().into_inner()),
            writer.reborrow(),
        )
        .map_err(|e| anyhow!("monty start failed: {e}"))?;

    loop {
        match progress {
            RunProgress::Complete(result) => {
                return Ok(RunResponse {
                    output: output_buf,
                    result: result.into(),
                });
            }
            RunProgress::FunctionCall(call) => {
                let result = handle_function_call(
                    &call.function_name,
                    &call.args,
                    &call.kwargs,
                    progress_token,
                );
                progress = call
                    .resume(result, writer.reborrow())
                    .map_err(|e| anyhow!("monty resume after FunctionCall failed: {e}"))?;
            }
            RunProgress::OsCall(call) => {
                let result = handle_os_call(&call.function, &call.args, &call.kwargs);
                progress = call
                    .resume(result, writer.reborrow())
                    .map_err(|e| anyhow!("monty resume after OsCall failed: {e}"))?;
            }
            RunProgress::ResolveFutures(futures_state) => {
                let results: Vec<(u32, ExtFunctionResult)> = futures_state
                    .pending_call_ids()
                    .iter()
                    .map(|&id| (id, ExtFunctionResult::Return(MontyObject::None)))
                    .collect();
                progress = futures_state
                    .resume(results, writer.reborrow())
                    .map_err(|e| anyhow!("monty resume after ResolveFutures failed: {e}"))?;
            }
            RunProgress::NameLookup(lookup) => {
                progress = lookup
                    .resume(monty::NameLookupResult::Undefined, writer.reborrow())
                    .map_err(|e| anyhow!("monty resume after NameLookup failed: {e}"))?;
            }
        }
    }
}

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

    match run_monty(run_input, progress_token_ref) {
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

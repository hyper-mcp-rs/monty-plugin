use crate::types::{PluginMontyObject, RunArguments, RunResponse};
use anyhow::{Result, anyhow};
use monty::{
    ExtFunctionResult, LimitedTracker, MontyObject, MontyRun, OsFunction, PrintWriter, RunProgress,
};

pub(crate) fn run_monty(
    input: RunArguments,
    mut handle_function_call: impl FnMut(
        &str,
        &[MontyObject],
        &[(MontyObject, MontyObject)],
    ) -> MontyObject,
    mut handle_os_call: impl FnMut(
        &OsFunction,
        &[MontyObject],
        &[(MontyObject, MontyObject)],
    ) -> MontyObject,
) -> Result<RunResponse> {
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
                let result = handle_function_call(&call.function_name, &call.args, &call.kwargs);
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

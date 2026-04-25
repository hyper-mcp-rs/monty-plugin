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
    let mut writer = PrintWriter::CollectString(&mut output_buf);

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

#[cfg(test)]
mod tests {
    use super::*;
    use monty::{ExcType, OsFunction};
    use std::collections::HashMap;

    // ---------------------------------------------------------------------
    // Stub handlers
    // ---------------------------------------------------------------------

    /// Stub function-call handler that recognises a handful of test functions.
    ///
    /// Supported functions:
    /// - `add(a: int, b: int) -> int`
    /// - `greet(name: str) -> str`
    /// - `echo(*args) -> tuple`  — returns its positional args as a tuple
    /// - `fail() -> Exception`   — always raises RuntimeError
    /// - `concat(a, b, sep=", ") -> str` — joins two strings with a separator (kwarg)
    /// - `counter() -> int`      — returns an incrementing counter (requires &mut)
    fn stub_function_call(
        function_name: &str,
        args: &[MontyObject],
        kwargs: &[(MontyObject, MontyObject)],
    ) -> MontyObject {
        match function_name {
            "add" => {
                if let (Some(MontyObject::Int(a)), Some(MontyObject::Int(b))) =
                    (args.first(), args.get(1))
                {
                    MontyObject::Int(a + b)
                } else {
                    MontyObject::Exception {
                        exc_type: ExcType::TypeError,
                        arg: Some("add() requires two int arguments".into()),
                    }
                }
            }
            "greet" => {
                if let Some(MontyObject::String(name)) = args.first() {
                    MontyObject::String(format!("Hello, {name}!"))
                } else {
                    MontyObject::Exception {
                        exc_type: ExcType::TypeError,
                        arg: Some("greet() requires a string argument".into()),
                    }
                }
            }
            "echo" => MontyObject::Tuple(args.to_vec()),
            "fail" => MontyObject::Exception {
                exc_type: ExcType::RuntimeError,
                arg: Some("intentional failure".into()),
            },
            "concat" => {
                let a = match args.first() {
                    Some(MontyObject::String(s)) => s.clone(),
                    _ => {
                        return MontyObject::Exception {
                            exc_type: ExcType::TypeError,
                            arg: Some("concat() requires string arguments".into()),
                        };
                    }
                };
                let b = match args.get(1) {
                    Some(MontyObject::String(s)) => s.clone(),
                    _ => {
                        return MontyObject::Exception {
                            exc_type: ExcType::TypeError,
                            arg: Some("concat() requires string arguments".into()),
                        };
                    }
                };
                let sep = kwargs
                    .iter()
                    .find(|(k, _)| matches!(k, MontyObject::String(s) if s == "sep"))
                    .and_then(|(_, v)| {
                        if let MontyObject::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| ", ".into());
                MontyObject::String(format!("{a}{sep}{b}"))
            }
            other => MontyObject::Exception {
                exc_type: ExcType::RuntimeError,
                arg: Some(format!("unknown function: {other}")),
            },
        }
    }

    /// Stub OS-call handler that supports a minimal virtual filesystem.
    ///
    /// Supported operations:
    /// - `ReadText`  — returns canned content for `/hello.txt`, errors otherwise
    /// - `Exists`    — returns `True` for `/hello.txt`
    /// - `IsFile`    — returns `True` for `/hello.txt`
    /// - `IsDir`     — returns `True` for `/data`
    /// - everything else → OSError
    fn stub_os_call(
        function: &OsFunction,
        args: &[MontyObject],
        _kwargs: &[(MontyObject, MontyObject)],
    ) -> MontyObject {
        let path = args.first().and_then(|a| match a {
            MontyObject::Path(p) => Some(p.as_str()),
            MontyObject::String(s) => Some(s.as_str()),
            _ => None,
        });

        match function {
            OsFunction::ReadText => match path {
                Some("/hello.txt") => MontyObject::String("hello world".into()),
                Some(p) => MontyObject::Exception {
                    exc_type: ExcType::OSError,
                    arg: Some(format!("file not found: {p}")),
                },
                None => MontyObject::Exception {
                    exc_type: ExcType::OSError,
                    arg: Some("read_text: no path provided".into()),
                },
            },
            OsFunction::Exists => match path {
                Some("/hello.txt" | "/data") => MontyObject::Bool(true),
                _ => MontyObject::Bool(false),
            },
            OsFunction::IsFile => match path {
                Some("/hello.txt") => MontyObject::Bool(true),
                _ => MontyObject::Bool(false),
            },
            OsFunction::IsDir => match path {
                Some("/data") => MontyObject::Bool(true),
                _ => MontyObject::Bool(false),
            },
            OsFunction::WriteText => MontyObject::None,
            OsFunction::Iterdir => match path {
                Some("/data") => MontyObject::List(vec![
                    MontyObject::String("a.txt".into()),
                    MontyObject::String("b.txt".into()),
                ]),
                _ => MontyObject::List(vec![]),
            },
            other => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("unsupported OS call in test: {other:?}")),
            },
        }
    }

    // ---------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------

    /// Build a `RunArguments` with no inputs and no resource limits.
    fn code_only(code: &str) -> RunArguments {
        RunArguments {
            code: code.into(),
            inputs: HashMap::new(),
            resource_limits: None,
        }
    }

    /// Build a `RunArguments` with the given inputs.
    fn with_inputs(code: &str, inputs: Vec<(&str, MontyObject)>) -> RunArguments {
        RunArguments {
            code: code.into(),
            inputs: inputs
                .into_iter()
                .map(|(k, v)| (k.to_string(), PluginMontyObject(v)))
                .collect(),
            resource_limits: None,
        }
    }

    /// Shorthand: run code through `run_monty` with stub handlers.
    fn run(input: RunArguments) -> Result<RunResponse> {
        run_monty(input, stub_function_call, stub_os_call)
    }

    /// Assert the result is a specific `MontyObject`.
    fn assert_result(resp: &RunResponse, expected: &MontyObject) {
        assert_eq!(resp.result.as_ref(), expected);
    }

    /// Assert the result is an exception whose type matches.
    fn assert_exception(resp: &RunResponse, expected_type: ExcType) {
        match resp.result.as_ref() {
            MontyObject::Exception { exc_type, .. } => {
                assert_eq!(*exc_type, expected_type, "wrong exception type");
            }
            other => panic!("expected Exception({expected_type:?}), got {other:?}"),
        }
    }

    /// Assert that `run_monty` returned an `Err` whose message contains the given substring.
    fn assert_err_contains(result: Result<RunResponse>, expected_substr: &str) {
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains(expected_substr),
                    "error message {msg:?} does not contain {expected_substr:?}"
                );
            }
            Ok(resp) => panic!(
                "expected Err containing {expected_substr:?}, got Ok with result {:?}",
                resp.result
            ),
        }
    }

    // =====================================================================
    // Basic expressions & return types
    // =====================================================================

    #[test]
    fn returns_none_for_statements() {
        let resp = run(code_only("x = 1")).unwrap();
        assert_result(&resp, &MontyObject::None);
    }

    #[test]
    fn returns_int() {
        let resp = run(code_only("1 + 2")).unwrap();
        assert_result(&resp, &MontyObject::Int(3));
    }

    #[test]
    fn returns_negative_int() {
        let resp = run(code_only("-42")).unwrap();
        assert_result(&resp, &MontyObject::Int(-42));
    }

    #[test]
    fn returns_float() {
        let resp = run(code_only("1.5 + 2.25")).unwrap();
        assert_result(&resp, &MontyObject::Float(3.75));
    }

    #[test]
    fn returns_bool_true() {
        let resp = run(code_only("3 > 2")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn returns_bool_false() {
        let resp = run(code_only("3 < 2")).unwrap();
        assert_result(&resp, &MontyObject::Bool(false));
    }

    #[test]
    fn returns_string() {
        let resp = run(code_only("'hello' + ' ' + 'world'")).unwrap();
        assert_result(&resp, &MontyObject::String("hello world".into()));
    }

    #[test]
    fn returns_none_literal() {
        let resp = run(code_only("None")).unwrap();
        assert_result(&resp, &MontyObject::None);
    }

    #[test]
    fn returns_list() {
        let resp = run(code_only("[1, 2, 3]")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(1),
                MontyObject::Int(2),
                MontyObject::Int(3),
            ]),
        );
    }

    #[test]
    fn returns_tuple() {
        let resp = run(code_only("(1, 'two', 3.0)")).unwrap();
        assert_result(
            &resp,
            &MontyObject::Tuple(vec![
                MontyObject::Int(1),
                MontyObject::String("two".into()),
                MontyObject::Float(3.0),
            ]),
        );
    }

    #[test]
    fn returns_dict() {
        let resp = run(code_only("{'a': 1}")).unwrap();
        match resp.result.as_ref() {
            MontyObject::Dict(pairs) => {
                let items: Vec<_> = pairs.into_iter().collect();
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].0, MontyObject::String("a".into()));
                assert_eq!(items[0].1, MontyObject::Int(1));
            }
            other => panic!("expected Dict, got {other:?}"),
        }
    }

    #[test]
    fn returns_empty_list() {
        let resp = run(code_only("[]")).unwrap();
        assert_result(&resp, &MontyObject::List(vec![]));
    }

    #[test]
    fn returns_bytes() {
        let resp = run(code_only("b'abc'")).unwrap();
        assert_result(&resp, &MontyObject::Bytes(b"abc".to_vec()));
    }

    // =====================================================================
    // Print / stdout capture
    // =====================================================================

    #[test]
    fn captures_print_output() {
        let resp = run(code_only("print('hello')")).unwrap();
        assert_eq!(resp.output, "hello\n");
    }

    #[test]
    fn captures_multiple_prints() {
        let resp = run(code_only("print('a')\nprint('b')\nprint('c')")).unwrap();
        assert_eq!(resp.output, "a\nb\nc\n");
    }

    #[test]
    fn print_with_sep_and_end() {
        let resp = run(code_only("print(1, 2, 3, sep='-', end='!')")).unwrap();
        assert_eq!(resp.output, "1-2-3!");
    }

    #[test]
    fn no_output_when_no_print() {
        let resp = run(code_only("1 + 1")).unwrap();
        assert!(resp.output.is_empty());
    }

    // =====================================================================
    // Inputs
    // =====================================================================

    #[test]
    fn single_int_input() {
        let resp = run(with_inputs("x + 1", vec![("x", MontyObject::Int(41))])).unwrap();
        assert_result(&resp, &MontyObject::Int(42));
    }

    #[test]
    fn single_string_input() {
        let resp = run(with_inputs(
            "'Hello, ' + name",
            vec![("name", MontyObject::String("World".into()))],
        ))
        .unwrap();
        assert_result(&resp, &MontyObject::String("Hello, World".into()));
    }

    #[test]
    fn multiple_inputs() {
        let resp = run(with_inputs(
            "a + b",
            vec![("a", MontyObject::Int(10)), ("b", MontyObject::Int(20))],
        ))
        .unwrap();
        assert_result(&resp, &MontyObject::Int(30));
    }

    #[test]
    fn list_input() {
        let resp = run(with_inputs(
            "len(items)",
            vec![(
                "items",
                MontyObject::List(vec![
                    MontyObject::Int(1),
                    MontyObject::Int(2),
                    MontyObject::Int(3),
                ]),
            )],
        ))
        .unwrap();
        assert_result(&resp, &MontyObject::Int(3));
    }

    #[test]
    fn bool_input() {
        let resp = run(with_inputs(
            "not flag",
            vec![("flag", MontyObject::Bool(true))],
        ))
        .unwrap();
        assert_result(&resp, &MontyObject::Bool(false));
    }

    #[test]
    fn none_input() {
        let resp = run(with_inputs("x is None", vec![("x", MontyObject::None)])).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    // =====================================================================
    // Control flow
    // =====================================================================

    #[test]
    fn if_else_true_branch() {
        let code = "\
if True:
    result = 'yes'
else:
    result = 'no'
result";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("yes".into()));
    }

    #[test]
    fn if_else_false_branch() {
        let code = "\
if False:
    result = 'yes'
else:
    result = 'no'
result";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("no".into()));
    }

    #[test]
    fn for_loop_sum() {
        let code = "\
total = 0
for i in range(5):
    total += i
total";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(10));
    }

    #[test]
    fn while_loop() {
        let code = "\
n = 0
while n < 10:
    n += 1
n";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(10));
    }

    #[test]
    fn list_comprehension() {
        let code = "[x * 2 for x in range(4)]";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(0),
                MontyObject::Int(2),
                MontyObject::Int(4),
                MontyObject::Int(6),
            ]),
        );
    }

    #[test]
    fn nested_loops() {
        let code = "\
pairs = []
for i in range(3):
    for j in range(2):
        pairs.append((i, j))
len(pairs)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(6));
    }

    // =====================================================================
    // Functions defined in Python
    // =====================================================================

    #[test]
    fn python_function_definition_and_call() {
        let code = "\
def square(n):
    return n * n
square(7)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(49));
    }

    #[test]
    fn recursive_function() {
        let code = "\
def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
fib(10)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(55));
    }

    #[test]
    fn function_with_default_args() {
        let code = "\
def greet(name, greeting='Hello'):
    return f'{greeting}, {name}!'
greet('World')";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("Hello, World!".into()));
    }

    #[test]
    fn lambda_expression() {
        let code = "\
double = lambda x: x * 2
double(21)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(42));
    }

    // =====================================================================
    // External function calls (via stub handler)
    // =====================================================================

    #[test]
    fn external_function_add() {
        let resp = run(code_only("add(3, 4)")).unwrap();
        assert_result(&resp, &MontyObject::Int(7));
    }

    #[test]
    fn external_function_greet() {
        let resp = run(code_only("greet('Alice')")).unwrap();
        assert_result(&resp, &MontyObject::String("Hello, Alice!".into()));
    }

    #[test]
    fn external_function_echo() {
        let resp = run(code_only("echo(1, 'two', True)")).unwrap();
        assert_result(
            &resp,
            &MontyObject::Tuple(vec![
                MontyObject::Int(1),
                MontyObject::String("two".into()),
                MontyObject::Bool(true),
            ]),
        );
    }

    #[test]
    fn external_function_echo_empty() {
        let resp = run(code_only("echo()")).unwrap();
        assert_result(&resp, &MontyObject::Tuple(vec![]));
    }

    #[test]
    fn external_function_chained_calls() {
        let code = "add(add(1, 2), add(3, 4))";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(10));
    }

    #[test]
    fn external_function_result_in_expression() {
        let code = "add(10, 20) * 2";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(60));
    }

    #[test]
    fn external_function_called_in_loop() {
        let code = "\
total = 0
for i in range(5):
    total = add(total, i)
total";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(10));
    }

    #[test]
    fn external_function_with_kwargs() {
        let code = "concat('hello', 'world', sep=' ')";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("hello world".into()));
    }

    #[test]
    fn external_function_with_default_kwarg() {
        let code = "concat('hello', 'world')";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("hello, world".into()));
    }

    #[test]
    fn external_function_raises_exception() {
        let resp = run(code_only("fail()")).unwrap();
        assert_exception(&resp, ExcType::RuntimeError);
    }

    #[test]
    fn external_function_exception_returned_as_value() {
        // MontyObject::Exception returned from a handler is a *value*, not a raised
        // exception, so the code continues past the call and the exception object
        // becomes the expression result.
        let code = "\
result = fail()
result";
        let resp = run(code_only(code)).unwrap();
        assert_exception(&resp, ExcType::RuntimeError);
    }

    #[test]
    fn external_function_wrong_arg_types() {
        let resp = run(code_only("add('a', 'b')")).unwrap();
        assert_exception(&resp, ExcType::TypeError);
    }

    #[test]
    fn external_function_with_input() {
        let resp = run(with_inputs(
            "add(x, 100)",
            vec![("x", MontyObject::Int(42))],
        ))
        .unwrap();
        assert_result(&resp, &MontyObject::Int(142));
    }

    // =====================================================================
    // External function calls (with closure state)
    // =====================================================================

    #[test]
    fn function_handler_with_mutable_state() {
        let mut counter = 0i64;
        let input = code_only("counter() + counter() + counter()");
        let resp = run_monty(
            input,
            |name, _args, _kwargs| match name {
                "counter" => {
                    counter += 1;
                    MontyObject::Int(counter)
                }
                _ => MontyObject::None,
            },
            stub_os_call,
        )
        .unwrap();
        // 1 + 2 + 3
        assert_result(&resp, &MontyObject::Int(6));
    }

    // =====================================================================
    // OS calls (via stub handler)
    // =====================================================================

    #[test]
    fn os_call_path_read_text() {
        let code = "\
from pathlib import Path
Path('/hello.txt').read_text()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("hello world".into()));
    }

    #[test]
    fn os_call_path_exists_true() {
        let code = "\
from pathlib import Path
Path('/hello.txt').exists()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn os_call_path_exists_false() {
        let code = "\
from pathlib import Path
Path('/nope.txt').exists()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Bool(false));
    }

    #[test]
    fn os_call_path_is_file() {
        let code = "\
from pathlib import Path
Path('/hello.txt').is_file()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn os_call_path_is_dir() {
        let code = "\
from pathlib import Path
Path('/data').is_dir()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn os_call_path_is_dir_false_for_file() {
        let code = "\
from pathlib import Path
Path('/hello.txt').is_dir()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Bool(false));
    }

    #[test]
    fn os_call_read_text_nonexistent() {
        let code = "\
from pathlib import Path
Path('/missing.txt').read_text()";
        let resp = run(code_only(code)).unwrap();
        assert_exception(&resp, ExcType::OSError);
    }

    #[test]
    fn os_call_read_and_process() {
        let code = "\
from pathlib import Path
text = Path('/hello.txt').read_text()
text.upper()";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("HELLO WORLD".into()));
    }

    #[test]
    fn os_call_iterdir() {
        let code = "\
from pathlib import Path
sorted(Path('/data').iterdir())";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::String("a.txt".into()),
                MontyObject::String("b.txt".into()),
            ]),
        );
    }

    #[test]
    fn os_call_conditional_on_exists() {
        let code = "\
from pathlib import Path
p = Path('/hello.txt')
if p.exists():
    result = p.read_text()
else:
    result = 'not found'
result";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("hello world".into()));
    }

    // =====================================================================
    // Error handling
    // =====================================================================

    #[test]
    fn syntax_error_returns_err() {
        let result = run(code_only("def +++"));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("parse") || msg.contains("syntax") || msg.contains("Syntax"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn division_by_zero() {
        assert_err_contains(run(code_only("1 / 0")), "ZeroDivisionError");
    }

    #[test]
    fn type_error_on_bad_op() {
        assert_err_contains(run(code_only("'hello' + 42")), "TypeError");
    }

    #[test]
    fn index_error() {
        assert_err_contains(run(code_only("[1, 2, 3][10]")), "IndexError");
    }

    #[test]
    fn key_error() {
        assert_err_contains(run(code_only("{'a': 1}['b']")), "KeyError");
    }

    #[test]
    fn attribute_error() {
        assert_err_contains(run(code_only("(42).nonexistent")), "AttributeError");
    }

    #[test]
    fn value_error() {
        assert_err_contains(run(code_only("int('not_a_number')")), "ValueError");
    }

    #[test]
    fn try_except_catches_zero_division() {
        let code = "\
try:
    1 / 0
except ZeroDivisionError:
    result = 'caught'
result";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("caught".into()));
    }

    #[test]
    fn try_except_finally() {
        let code = "\
log = []
try:
    log.append('try')
    x = 1 / 0
except ZeroDivisionError:
    log.append('except')
finally:
    log.append('finally')
log";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::String("try".into()),
                MontyObject::String("except".into()),
                MontyObject::String("finally".into()),
            ]),
        );
    }

    // =====================================================================
    // String operations & f-strings
    // =====================================================================

    #[test]
    fn fstring_formatting() {
        let code = "\
name = 'World'
f'Hello, {name}!'";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("Hello, World!".into()));
    }

    #[test]
    fn fstring_with_expression() {
        let resp = run(code_only("f'{2 + 3}'")).unwrap();
        assert_result(&resp, &MontyObject::String("5".into()));
    }

    #[test]
    fn string_methods() {
        let resp = run(code_only("'hello world'.upper()")).unwrap();
        assert_result(&resp, &MontyObject::String("HELLO WORLD".into()));
    }

    #[test]
    fn string_split() {
        let resp = run(code_only("'a,b,c'.split(',')")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::String("a".into()),
                MontyObject::String("b".into()),
                MontyObject::String("c".into()),
            ]),
        );
    }

    #[test]
    fn string_join() {
        let resp = run(code_only("'-'.join(['x', 'y', 'z'])")).unwrap();
        assert_result(&resp, &MontyObject::String("x-y-z".into()));
    }

    #[test]
    fn string_strip() {
        let resp = run(code_only("'  hello  '.strip()")).unwrap();
        assert_result(&resp, &MontyObject::String("hello".into()));
    }

    #[test]
    fn string_startswith() {
        let resp = run(code_only("'hello world'.startswith('hello')")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn string_replace() {
        let resp = run(code_only("'aabbcc'.replace('bb', 'XX')")).unwrap();
        assert_result(&resp, &MontyObject::String("aaXXcc".into()));
    }

    // =====================================================================
    // Builtin functions
    // =====================================================================

    #[test]
    fn builtin_len() {
        let resp = run(code_only("len([1, 2, 3, 4])")).unwrap();
        assert_result(&resp, &MontyObject::Int(4));
    }

    #[test]
    fn builtin_abs() {
        let resp = run(code_only("abs(-7)")).unwrap();
        assert_result(&resp, &MontyObject::Int(7));
    }

    #[test]
    fn builtin_min_max() {
        let resp = run(code_only("(min(3, 1, 2), max(3, 1, 2))")).unwrap();
        assert_result(
            &resp,
            &MontyObject::Tuple(vec![MontyObject::Int(1), MontyObject::Int(3)]),
        );
    }

    #[test]
    fn builtin_sorted() {
        let resp = run(code_only("sorted([3, 1, 4, 1, 5])")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(1),
                MontyObject::Int(1),
                MontyObject::Int(3),
                MontyObject::Int(4),
                MontyObject::Int(5),
            ]),
        );
    }

    #[test]
    fn builtin_sum() {
        let resp = run(code_only("sum([10, 20, 30])")).unwrap();
        assert_result(&resp, &MontyObject::Int(60));
    }

    #[test]
    fn builtin_range_list() {
        let resp = run(code_only("list(range(5))")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(0),
                MontyObject::Int(1),
                MontyObject::Int(2),
                MontyObject::Int(3),
                MontyObject::Int(4),
            ]),
        );
    }

    #[test]
    fn builtin_isinstance() {
        let resp = run(code_only("isinstance(42, int)")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn builtin_int_conversion() {
        let resp = run(code_only("int('123')")).unwrap();
        assert_result(&resp, &MontyObject::Int(123));
    }

    #[test]
    fn builtin_str_conversion() {
        let resp = run(code_only("str(42)")).unwrap();
        assert_result(&resp, &MontyObject::String("42".into()));
    }

    #[test]
    fn builtin_bool_conversion() {
        let resp = run(code_only("(bool(0), bool(1))")).unwrap();
        assert_result(
            &resp,
            &MontyObject::Tuple(vec![MontyObject::Bool(false), MontyObject::Bool(true)]),
        );
    }

    #[test]
    fn builtin_enumerate() {
        let code = "list(enumerate(['a', 'b']))";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Tuple(vec![MontyObject::Int(0), MontyObject::String("a".into())]),
                MontyObject::Tuple(vec![MontyObject::Int(1), MontyObject::String("b".into())]),
            ]),
        );
    }

    #[test]
    fn builtin_zip() {
        let code = "list(zip([1, 2], ['a', 'b']))";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Tuple(vec![MontyObject::Int(1), MontyObject::String("a".into())]),
                MontyObject::Tuple(vec![MontyObject::Int(2), MontyObject::String("b".into())]),
            ]),
        );
    }

    // =====================================================================
    // Dict operations
    // =====================================================================

    #[test]
    fn dict_access() {
        let code = "{'x': 10, 'y': 20}['x']";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(10));
    }

    #[test]
    fn dict_get_with_default() {
        let code = "{'a': 1}.get('b', 99)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(99));
    }

    #[test]
    fn dict_keys_iteration() {
        let code = "sorted({'b': 2, 'a': 1, 'c': 3}.keys())";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::String("a".into()),
                MontyObject::String("b".into()),
                MontyObject::String("c".into()),
            ]),
        );
    }

    #[test]
    fn dict_in_operator() {
        let code = "'a' in {'a': 1, 'b': 2}";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    // =====================================================================
    // List operations
    // =====================================================================

    #[test]
    fn list_append_and_len() {
        let code = "\
xs = [1, 2]
xs.append(3)
xs.append(4)
len(xs)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(4));
    }

    #[test]
    fn list_slicing() {
        let resp = run(code_only("[10, 20, 30, 40, 50][1:4]")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(20),
                MontyObject::Int(30),
                MontyObject::Int(40),
            ]),
        );
    }

    #[test]
    fn list_in_operator() {
        let resp = run(code_only("3 in [1, 2, 3, 4]")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn list_not_in_operator() {
        let resp = run(code_only("9 not in [1, 2, 3]")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn list_concatenation() {
        let resp = run(code_only("[1, 2] + [3, 4]")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(1),
                MontyObject::Int(2),
                MontyObject::Int(3),
                MontyObject::Int(4),
            ]),
        );
    }

    #[test]
    fn list_repetition() {
        let resp = run(code_only("[0] * 3")).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(0),
                MontyObject::Int(0),
                MontyObject::Int(0),
            ]),
        );
    }

    // =====================================================================
    // Arithmetic edge cases
    // =====================================================================

    #[test]
    fn integer_division() {
        let resp = run(code_only("7 // 2")).unwrap();
        assert_result(&resp, &MontyObject::Int(3));
    }

    #[test]
    fn modulo() {
        let resp = run(code_only("17 % 5")).unwrap();
        assert_result(&resp, &MontyObject::Int(2));
    }

    #[test]
    fn exponentiation() {
        let resp = run(code_only("2 ** 10")).unwrap();
        assert_result(&resp, &MontyObject::Int(1024));
    }

    #[test]
    fn float_division() {
        let resp = run(code_only("7 / 2")).unwrap();
        assert_result(&resp, &MontyObject::Float(3.5));
    }

    #[test]
    fn negative_indexing() {
        let resp = run(code_only("[10, 20, 30][-1]")).unwrap();
        assert_result(&resp, &MontyObject::Int(30));
    }

    // =====================================================================
    // Multiple external + OS calls in one run
    // =====================================================================

    #[test]
    fn mixed_external_and_os_calls() {
        let code = "\
from pathlib import Path
text = Path('/hello.txt').read_text()
greet(text)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("Hello, hello world!".into()));
    }

    #[test]
    fn external_call_with_print() {
        let code = "\
result = add(10, 20)
print(f'result is {result}')
result";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(30));
        assert_eq!(resp.output, "result is 30\n");
    }

    // =====================================================================
    // Unpacking and multiple assignment
    // =====================================================================

    #[test]
    fn tuple_unpacking() {
        let code = "\
a, b, c = (1, 2, 3)
a + b + c";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(6));
    }

    #[test]
    fn sequential_assignment() {
        let code = "\
x = 42
y = x
x + y";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(84));
    }

    // =====================================================================
    // Boolean logic
    // =====================================================================

    #[test]
    fn boolean_and() {
        let resp = run(code_only("True and False")).unwrap();
        assert_result(&resp, &MontyObject::Bool(false));
    }

    #[test]
    fn boolean_or() {
        let resp = run(code_only("False or True")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn boolean_not() {
        let resp = run(code_only("not False")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    #[test]
    fn chained_comparison() {
        let resp = run(code_only("1 < 2 < 3")).unwrap();
        assert_result(&resp, &MontyObject::Bool(true));
    }

    // =====================================================================
    // Complex / integration-style
    // =====================================================================

    #[test]
    fn build_dict_from_external_calls() {
        let code = "\
result = {}
for name in ['Alice', 'Bob']:
    result[name] = greet(name)
result['Alice']";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("Hello, Alice!".into()));
    }

    #[test]
    fn filter_with_external_and_os() {
        let code = "\
from pathlib import Path
files = Path('/data').iterdir()
txt_files = [f for f in files if f.endswith('.txt')]
len(txt_files)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(2));
    }

    #[test]
    fn walrus_operator() {
        let code = "\
data = [1, 2, 3, 4, 5]
result = [y for x in data if (y := x * 2) > 4]
result";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(6),
                MontyObject::Int(8),
                MontyObject::Int(10),
            ]),
        );
    }

    #[test]
    fn ternary_expression() {
        let code = "'even' if 4 % 2 == 0 else 'odd'";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::String("even".into()));
    }

    #[test]
    fn nested_function_closures() {
        let code = "\
def make_adder(n):
    def adder(x):
        return x + n
    return adder
add5 = make_adder(5)
add5(10)";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(15));
    }

    #[test]
    fn map_with_lambda() {
        let code = "list(map(lambda x: x ** 2, [1, 2, 3, 4]))";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(1),
                MontyObject::Int(4),
                MontyObject::Int(9),
                MontyObject::Int(16),
            ]),
        );
    }

    #[test]
    fn filter_with_lambda() {
        let code = "list(filter(lambda x: x > 2, [1, 2, 3, 4, 5]))";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(3),
                MontyObject::Int(4),
                MontyObject::Int(5),
            ]),
        );
    }

    #[test]
    fn dict_comprehension() {
        let code = "{k: k * 10 for k in range(3)}";
        let resp = run(code_only(code)).unwrap();
        match resp.result.as_ref() {
            MontyObject::Dict(pairs) => {
                let items: Vec<_> = pairs.into_iter().collect();
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected Dict, got {other:?}"),
        }
    }

    #[test]
    fn set_comprehension() {
        let code = "len({x % 3 for x in range(10)})";
        let resp = run(code_only(code)).unwrap();
        assert_result(&resp, &MontyObject::Int(3));
    }

    #[test]
    fn generator_with_sum() {
        let code = "sum(x * x for x in range(5))";
        let resp = run(code_only(code)).unwrap();
        // 0 + 1 + 4 + 9 + 16 = 30
        assert_result(&resp, &MontyObject::Int(30));
    }

    #[test]
    fn multiline_with_print_and_return() {
        let code = "\
results = []
for i in range(5):
    val = add(i, i)
    print(f'{i} + {i} = {val}')
    results.append(val)
results";
        let resp = run(code_only(code)).unwrap();
        assert_result(
            &resp,
            &MontyObject::List(vec![
                MontyObject::Int(0),
                MontyObject::Int(2),
                MontyObject::Int(4),
                MontyObject::Int(6),
                MontyObject::Int(8),
            ]),
        );
        assert_eq!(
            resp.output,
            "0 + 0 = 0\n1 + 1 = 2\n2 + 2 = 4\n3 + 3 = 6\n4 + 4 = 8\n"
        );
    }
}

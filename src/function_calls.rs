use crate::pdk::{imports, types::*};
use extism_pdk::{HttpRequest, http};
use monty::{ExcType, MontyObject};
use std::collections::BTreeMap;

/// Markdown description of the built-in functions available to Python code.
pub(crate) const BUILTIN_FUNCTIONS_DESCRIPTION: &str = "\
Built-in functions:\n\
- `http_request(url: str, method: str | None = None, headers: dict[str, str] | None = None, body: str | bytes | None = None) -> tuple[int, dict[str, str], str | bytes]` — make HTTP requests\n\
- `notify_progress(message: str | None, progress: int | float, total: int | float | None = None) -> None` — report progress";

/// Dispatch an external function call by name.
pub(crate) fn handle_function_call(
    function_name: &str,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
    progress_token: Option<&ProgressToken>,
) -> MontyObject {
    match function_name {
        EXT_HTTP_REQUEST => handle_http_request(args, kwargs),
        EXT_NOTIFY_PROGRESS => handle_notify_progress(args, kwargs, progress_token),
        other => MontyObject::Exception {
            exc_type: ExcType::RuntimeError,
            arg: Some(format!("unknown external function called: {other}")),
        },
    }
}

const EXT_HTTP_REQUEST: &str = "http_request";
const EXT_NOTIFY_PROGRESS: &str = "notify_progress";

/// Handle the `http_request` Monty external function.
///
/// Parameters (positional or keyword):
/// - `url`: str (required)
/// - `method`: str | None (optional — defaults to `None` / GET)
/// - `headers`: dict[str, str] | None (optional — defaults to empty)
/// - `body`: str | bytes | None (optional)
fn handle_http_request(args: &[MontyObject], kwargs: &[(MontyObject, MontyObject)]) -> MontyObject {
    // -- url (required) --
    let url = match resolve_str_arg(args, 0, kwargs, "url", "http_request", None) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("http_request: missing required argument: 'url'".into()),
            };
        }
        Err(e) => return e,
    };

    // -- method (optional) --
    let method = match resolve_str_arg(args, 1, kwargs, "method", "http_request", None) {
        Ok(v) => v,
        Err(e) => return e,
    };

    // -- headers (optional) --
    let mut headers = BTreeMap::new();
    match resolve_arg(args, 2, kwargs, "headers") {
        Some(MontyObject::Dict(pairs)) => {
            for (hk, hv) in pairs {
                if let (MontyObject::String(hk_s), MontyObject::String(hv_s)) = (hk, hv) {
                    headers.insert(hk_s.clone(), hv_s.clone());
                }
            }
        }
        None => {}
        Some(_) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("http_request: 'headers' must be a dict".into()),
            };
        }
    }

    let request = HttpRequest {
        url,
        method,
        headers,
    };

    // -- body (optional) --
    let body: Option<Vec<u8>> = match resolve_arg(args, 3, kwargs, "body") {
        Some(MontyObject::String(s)) => Some(s.as_bytes().to_vec()),
        Some(MontyObject::Bytes(b)) => Some(b.clone()),
        None => None,
        Some(other) => match serde_json::to_vec(other) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                return MontyObject::Exception {
                    exc_type: ExcType::ValueError,
                    arg: Some(format!("http_request: failed to serialize body: {e}")),
                };
            }
        },
    };

    let response = match http::request(&request, body.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            return MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("http_request failed: {e}")),
            };
        }
    };

    // Build a tuple of (status, headers, body)
    let status = MontyObject::Int(response.status_code() as i64);

    let resp_headers: Vec<(MontyObject, MontyObject)> = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                MontyObject::String(k.clone()),
                MontyObject::String(v.clone()),
            )
        })
        .collect();

    let body_bytes = response.body();
    let body_obj = match String::from_utf8(body_bytes.clone()) {
        Ok(s) => MontyObject::String(s),
        Err(_) => MontyObject::Bytes(body_bytes),
    };

    MontyObject::Tuple(vec![
        status,
        MontyObject::Dict(resp_headers.into()),
        body_obj,
    ])
}

/// Handle the `notify_progress` Monty external function.
///
/// Parameters (positional or keyword):
/// - `message`: str | None (optional)
/// - `progress`: int | float (required)
/// - `total`: int | float | None (optional)
fn handle_notify_progress(
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
    progress_token: Option<&ProgressToken>,
) -> MontyObject {
    let Some(token) = progress_token else {
        // No progress token in the request context — silently skip the notification.
        return MontyObject::None;
    };

    // -- message (optional) --
    let message = match resolve_str_arg(args, 0, kwargs, "message", "notify_progress", None) {
        Ok(v) => v,
        Err(e) => return e,
    };

    // -- progress (required) --
    let progress = match resolve_arg(args, 1, kwargs, "progress") {
        Some(MontyObject::Int(n)) => *n as f64,
        Some(MontyObject::Float(f)) => *f,
        Some(_) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("notify_progress: 'progress' must be a number".into()),
            };
        }
        None => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("notify_progress: missing required argument: 'progress'".into()),
            };
        }
    };

    // -- total (optional) --
    let total = match resolve_arg(args, 2, kwargs, "total") {
        Some(MontyObject::Int(n)) => Some(*n as f64),
        Some(MontyObject::Float(f)) => Some(*f),
        None => None,
        Some(_) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("notify_progress: 'total' must be a number".into()),
            };
        }
    };

    let param = ProgressNotificationParam {
        message,
        progress,
        progress_token: token.clone(),
        total,
    };

    if let Err(e) = imports::notify_progress(param) {
        return MontyObject::Exception {
            exc_type: ExcType::RuntimeError,
            arg: Some(format!("notify_progress host call failed: {e}")),
        };
    }

    MontyObject::None
}
// ---------------------------------------------------------------------------
// Python-argument resolution helpers
// ---------------------------------------------------------------------------

/// Look up a keyword argument by name from a kwargs slice.
fn get_kwarg<'a>(kwargs: &'a [(MontyObject, MontyObject)], name: &str) -> Option<&'a MontyObject> {
    for (k, v) in kwargs {
        if let MontyObject::String(key) = k
            && key == name
        {
            return Some(v);
        }
    }
    None
}

/// Resolve a parameter from a positional arg or a kwarg fallback.
/// Returns `None` if neither is present or the value is `MontyObject::None`.
fn resolve_arg<'a>(
    args: &'a [MontyObject],
    index: usize,
    kwargs: &'a [(MontyObject, MontyObject)],
    name: &str,
) -> Option<&'a MontyObject> {
    match args.get(index) {
        Some(MontyObject::None) | None => {}
        Some(v) => return Some(v),
    }
    match get_kwarg(kwargs, name) {
        Some(MontyObject::None) | None => None,
        Some(v) => Some(v),
    }
}

/// Resolve a string parameter from a positional arg or a kwarg fallback.
///
/// Returns:
/// - `Ok(Some(s))` — a string value was found (or the default was used)
/// - `Ok(None)` — not present (or `None`) and no default
/// - `Err(exception)` — present but wrong type (`MontyObject::Exception` with `TypeError`)
fn resolve_str_arg(
    args: &[MontyObject],
    index: usize,
    kwargs: &[(MontyObject, MontyObject)],
    name: &str,
    func_name: &str,
    default: Option<&str>,
) -> Result<Option<String>, MontyObject> {
    match resolve_arg(args, index, kwargs, name) {
        Some(MontyObject::String(s)) => Ok(Some(s.clone())),
        Some(MontyObject::None) | None => Ok(default.map(String::from)),
        Some(_) => Err(MontyObject::Exception {
            exc_type: ExcType::TypeError,
            arg: Some(format!("{func_name}: '{name}' must be a string")),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────

    fn kwarg(name: &str, val: MontyObject) -> (MontyObject, MontyObject) {
        (MontyObject::String(name.to_string()), val)
    }

    fn s(val: &str) -> MontyObject {
        MontyObject::String(val.to_string())
    }

    fn is_type_error(obj: &MontyObject) -> bool {
        matches!(
            obj,
            MontyObject::Exception {
                exc_type: ExcType::TypeError,
                ..
            }
        )
    }

    // ── get_kwarg ───────────────────────────────────────────────────

    #[test]
    fn get_kwarg_found() {
        let kwargs = vec![kwarg("foo", MontyObject::Int(42))];
        let result = get_kwarg(&kwargs, "foo");
        assert!(matches!(result, Some(MontyObject::Int(42))));
    }

    #[test]
    fn get_kwarg_not_found() {
        let kwargs = vec![kwarg("foo", MontyObject::Int(42))];
        assert!(get_kwarg(&kwargs, "bar").is_none());
    }

    #[test]
    fn get_kwarg_empty_kwargs() {
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert!(get_kwarg(&kwargs, "anything").is_none());
    }

    #[test]
    fn get_kwarg_multiple_keys_returns_first() {
        let kwargs = vec![
            kwarg("x", MontyObject::Int(1)),
            kwarg("y", MontyObject::Int(2)),
            kwarg("x", MontyObject::Int(3)),
        ];
        // Should return the first match
        assert!(matches!(get_kwarg(&kwargs, "x"), Some(MontyObject::Int(1))));
        assert!(matches!(get_kwarg(&kwargs, "y"), Some(MontyObject::Int(2))));
    }

    #[test]
    fn get_kwarg_ignores_non_string_keys() {
        let kwargs = vec![(MontyObject::Int(1), MontyObject::Bool(true))];
        assert!(get_kwarg(&kwargs, "1").is_none());
    }

    // ── resolve_arg ─────────────────────────────────────────────────

    #[test]
    fn resolve_arg_from_positional() {
        let args = vec![MontyObject::Int(10)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_arg(&args, 0, &kwargs, "x");
        assert!(matches!(result, Some(MontyObject::Int(10))));
    }

    #[test]
    fn resolve_arg_from_kwarg_when_positional_missing() {
        let args: Vec<MontyObject> = vec![];
        let kwargs = vec![kwarg("x", MontyObject::Int(20))];
        let result = resolve_arg(&args, 0, &kwargs, "x");
        assert!(matches!(result, Some(MontyObject::Int(20))));
    }

    #[test]
    fn resolve_arg_from_kwarg_when_positional_is_none() {
        let args = vec![MontyObject::None];
        let kwargs = vec![kwarg("x", MontyObject::Int(30))];
        let result = resolve_arg(&args, 0, &kwargs, "x");
        assert!(matches!(result, Some(MontyObject::Int(30))));
    }

    #[test]
    fn resolve_arg_returns_none_when_both_absent() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert!(resolve_arg(&args, 0, &kwargs, "x").is_none());
    }

    #[test]
    fn resolve_arg_returns_none_when_both_are_none_obj() {
        let args = vec![MontyObject::None];
        let kwargs = vec![kwarg("x", MontyObject::None)];
        assert!(resolve_arg(&args, 0, &kwargs, "x").is_none());
    }

    #[test]
    fn resolve_arg_positional_takes_precedence_over_kwarg() {
        let args = vec![MontyObject::Int(1)];
        let kwargs = vec![kwarg("x", MontyObject::Int(2))];
        let result = resolve_arg(&args, 0, &kwargs, "x");
        assert!(matches!(result, Some(MontyObject::Int(1))));
    }

    #[test]
    fn resolve_arg_index_out_of_bounds_falls_through_to_kwarg() {
        let args = vec![MontyObject::Int(1)];
        let kwargs = vec![kwarg("y", MontyObject::Int(99))];
        // Index 5 is out of bounds for args
        let result = resolve_arg(&args, 5, &kwargs, "y");
        assert!(matches!(result, Some(MontyObject::Int(99))));
    }

    #[test]
    fn resolve_arg_skips_none_positional_to_kwarg() {
        // Simulates f(None, kwarg_b=42) where positional 0 is None
        let args = vec![MontyObject::None];
        let kwargs = vec![kwarg("a", MontyObject::Int(42))];
        let result = resolve_arg(&args, 0, &kwargs, "a");
        assert!(matches!(result, Some(MontyObject::Int(42))));
    }

    // ── resolve_str_arg ─────────────────────────────────────────────

    #[test]
    fn resolve_str_arg_from_positional() {
        let args = vec![s("hello")];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_str_arg(&args, 0, &kwargs, "name", "test_fn", None);
        assert_eq!(result.unwrap(), Some("hello".to_string()));
    }

    #[test]
    fn resolve_str_arg_from_kwarg() {
        let args: Vec<MontyObject> = vec![];
        let kwargs = vec![kwarg("name", s("world"))];
        let result = resolve_str_arg(&args, 0, &kwargs, "name", "test_fn", None);
        assert_eq!(result.unwrap(), Some("world".to_string()));
    }

    #[test]
    fn resolve_str_arg_absent_no_default() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_str_arg(&args, 0, &kwargs, "name", "test_fn", None);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn resolve_str_arg_absent_with_default() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_str_arg(&args, 0, &kwargs, "name", "test_fn", Some("fallback"));
        assert_eq!(result.unwrap(), Some("fallback".to_string()));
    }

    #[test]
    fn resolve_str_arg_wrong_type_returns_type_error() {
        let args = vec![MontyObject::Int(42)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let err = resolve_str_arg(&args, 0, &kwargs, "name", "my_func", None).unwrap_err();
        assert!(is_type_error(&err));
        if let MontyObject::Exception { arg, .. } = &err {
            let msg = arg.as_ref().unwrap();
            assert!(msg.contains("my_func"));
            assert!(msg.contains("'name'"));
            assert!(msg.contains("string"));
        }
    }

    #[test]
    fn resolve_str_arg_none_positional_uses_default() {
        let args = vec![MontyObject::None];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_str_arg(&args, 0, &kwargs, "name", "test_fn", Some("def"));
        assert_eq!(result.unwrap(), Some("def".to_string()));
    }

    #[test]
    fn resolve_str_arg_wrong_type_in_kwarg() {
        let args: Vec<MontyObject> = vec![];
        let kwargs = vec![kwarg("name", MontyObject::Bool(true))];
        let err = resolve_str_arg(&args, 0, &kwargs, "name", "fn_x", None).unwrap_err();
        assert!(is_type_error(&err));
    }
}

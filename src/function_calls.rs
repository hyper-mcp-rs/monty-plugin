use crate::pdk::{imports, types::*};
use crate::python_args::{resolve_arg, resolve_str_arg};
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

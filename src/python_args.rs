use monty::{ExcType, MontyObject};

/// Look up a keyword argument by name from a kwargs slice.
pub(crate) fn get_kwarg<'a>(
    kwargs: &'a [(MontyObject, MontyObject)],
    name: &str,
) -> Option<&'a MontyObject> {
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
pub(crate) fn resolve_arg<'a>(
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
pub(crate) fn resolve_str_arg(
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

/// Look up a boolean keyword argument by name, defaulting to `default` if absent.
///
/// Returns:
/// - `Ok(default)` — not present (or `None`)
/// - `Ok(b)` — a bool value was found
/// - `Err(exception)` — present but wrong type (`MontyObject::Exception` with `TypeError`)
pub(crate) fn get_bool_kwarg(
    kwargs: &[(MontyObject, MontyObject)],
    name: &str,
    default: bool,
    func_name: &str,
) -> Result<bool, MontyObject> {
    match get_kwarg(kwargs, name) {
        Some(MontyObject::Bool(b)) => Ok(*b),
        Some(MontyObject::None) | None => Ok(default),
        Some(_) => Err(MontyObject::Exception {
            exc_type: ExcType::TypeError,
            arg: Some(format!("{func_name}: '{name}' must be a bool")),
        }),
    }
}

/// Resolve an integer parameter from a positional arg or a kwarg fallback.
///
/// Returns:
/// - `Ok(Some(n))` — an int value was found (or the default was used)
/// - `Ok(None)` — not present (or `None`) and no default
/// - `Err(exception)` — present but wrong type (`MontyObject::Exception` with `TypeError`)
pub(crate) fn resolve_int_arg(
    args: &[MontyObject],
    index: usize,
    kwargs: &[(MontyObject, MontyObject)],
    name: &str,
    func_name: &str,
    default: Option<i64>,
) -> Result<Option<i64>, MontyObject> {
    match resolve_arg(args, index, kwargs, name) {
        Some(MontyObject::Int(n)) => Ok(Some(*n)),
        Some(MontyObject::None) | None => Ok(default),
        Some(_) => Err(MontyObject::Exception {
            exc_type: ExcType::TypeError,
            arg: Some(format!("{func_name}: '{name}' must be an int")),
        }),
    }
}

/// Resolve a boolean parameter from a positional arg or a kwarg fallback.
///
/// Returns:
/// - `Ok(default)` — not present (or `None`)
/// - `Ok(b)` — a bool value was found
/// - `Err(exception)` — present but wrong type (`MontyObject::Exception` with `TypeError`)
pub(crate) fn resolve_bool_arg(
    args: &[MontyObject],
    index: usize,
    kwargs: &[(MontyObject, MontyObject)],
    name: &str,
    func_name: &str,
    default: bool,
) -> Result<bool, MontyObject> {
    match resolve_arg(args, index, kwargs, name) {
        Some(MontyObject::Bool(b)) => Ok(*b),
        Some(MontyObject::None) | None => Ok(default),
        Some(_) => Err(MontyObject::Exception {
            exc_type: ExcType::TypeError,
            arg: Some(format!("{func_name}: '{name}' must be a bool")),
        }),
    }
}

/// Resolve a bytes parameter from a positional arg or a kwarg fallback.
///
/// Returns:
/// - `Ok(Some(b))` — a bytes value was found (or the default was used)
/// - `Ok(None)` — not present (or `None`) and no default
/// - `Err(exception)` — present but wrong type (`MontyObject::Exception` with `TypeError`)
pub(crate) fn resolve_bytes_arg<'a>(
    args: &'a [MontyObject],
    index: usize,
    kwargs: &'a [(MontyObject, MontyObject)],
    name: &str,
    func_name: &str,
) -> Result<Option<&'a Vec<u8>>, MontyObject> {
    match resolve_arg(args, index, kwargs, name) {
        Some(MontyObject::Bytes(b)) => Ok(Some(b)),
        Some(MontyObject::None) | None => Ok(None),
        Some(_) => Err(MontyObject::Exception {
            exc_type: ExcType::TypeError,
            arg: Some(format!("{func_name}: '{name}' must be bytes")),
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

    // ── get_bool_kwarg ──────────────────────────────────────────────

    #[test]
    fn get_bool_kwarg_found_true() {
        let kwargs = vec![kwarg("flag", MontyObject::Bool(true))];
        assert_eq!(
            get_bool_kwarg(&kwargs, "flag", false, "test_fn").unwrap(),
            true
        );
    }

    #[test]
    fn get_bool_kwarg_found_false() {
        let kwargs = vec![kwarg("flag", MontyObject::Bool(false))];
        assert_eq!(
            get_bool_kwarg(&kwargs, "flag", true, "test_fn").unwrap(),
            false
        );
    }

    #[test]
    fn get_bool_kwarg_absent_returns_default_true() {
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert_eq!(
            get_bool_kwarg(&kwargs, "flag", true, "test_fn").unwrap(),
            true
        );
    }

    #[test]
    fn get_bool_kwarg_absent_returns_default_false() {
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert_eq!(
            get_bool_kwarg(&kwargs, "flag", false, "test_fn").unwrap(),
            false
        );
    }

    #[test]
    fn get_bool_kwarg_none_value_returns_default() {
        let kwargs = vec![kwarg("flag", MontyObject::None)];
        assert_eq!(
            get_bool_kwarg(&kwargs, "flag", true, "test_fn").unwrap(),
            true
        );
    }

    #[test]
    fn get_bool_kwarg_wrong_type_returns_error() {
        let kwargs = vec![kwarg("flag", s("yes"))];
        let err = get_bool_kwarg(&kwargs, "flag", false, "my_func").unwrap_err();
        assert!(is_type_error(&err));
        if let MontyObject::Exception { arg, .. } = &err {
            let msg = arg.as_ref().unwrap();
            assert!(msg.contains("my_func"));
            assert!(msg.contains("'flag'"));
            assert!(msg.contains("bool"));
        }
    }

    // ── resolve_int_arg ─────────────────────────────────────────────

    #[test]
    fn resolve_int_arg_from_positional() {
        let args = vec![MontyObject::Int(7)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_int_arg(&args, 0, &kwargs, "count", "test_fn", None);
        assert_eq!(result.unwrap(), Some(7));
    }

    #[test]
    fn resolve_int_arg_from_kwarg() {
        let args: Vec<MontyObject> = vec![];
        let kwargs = vec![kwarg("count", MontyObject::Int(-3))];
        let result = resolve_int_arg(&args, 0, &kwargs, "count", "test_fn", None);
        assert_eq!(result.unwrap(), Some(-3));
    }

    #[test]
    fn resolve_int_arg_absent_no_default() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_int_arg(&args, 0, &kwargs, "count", "test_fn", None);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn resolve_int_arg_absent_with_default() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_int_arg(&args, 0, &kwargs, "count", "test_fn", Some(100));
        assert_eq!(result.unwrap(), Some(100));
    }

    #[test]
    fn resolve_int_arg_none_positional_uses_default() {
        let args = vec![MontyObject::None];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_int_arg(&args, 0, &kwargs, "count", "test_fn", Some(50));
        assert_eq!(result.unwrap(), Some(50));
    }

    #[test]
    fn resolve_int_arg_wrong_type_returns_error() {
        let args = vec![s("not_a_number")];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let err = resolve_int_arg(&args, 0, &kwargs, "count", "fn_z", None).unwrap_err();
        assert!(is_type_error(&err));
        if let MontyObject::Exception { arg, .. } = &err {
            let msg = arg.as_ref().unwrap();
            assert!(msg.contains("fn_z"));
            assert!(msg.contains("'count'"));
            assert!(msg.contains("int"));
        }
    }

    // ── resolve_bool_arg ────────────────────────────────────────────

    #[test]
    fn resolve_bool_arg_from_positional_true() {
        let args = vec![MontyObject::Bool(true)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert_eq!(
            resolve_bool_arg(&args, 0, &kwargs, "flag", "test_fn", false).unwrap(),
            true
        );
    }

    #[test]
    fn resolve_bool_arg_from_positional_false() {
        let args = vec![MontyObject::Bool(false)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert_eq!(
            resolve_bool_arg(&args, 0, &kwargs, "flag", "test_fn", true).unwrap(),
            false
        );
    }

    #[test]
    fn resolve_bool_arg_from_kwarg() {
        let args: Vec<MontyObject> = vec![];
        let kwargs = vec![kwarg("flag", MontyObject::Bool(true))];
        assert_eq!(
            resolve_bool_arg(&args, 0, &kwargs, "flag", "test_fn", false).unwrap(),
            true
        );
    }

    #[test]
    fn resolve_bool_arg_absent_returns_default() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert_eq!(
            resolve_bool_arg(&args, 0, &kwargs, "flag", "test_fn", true).unwrap(),
            true
        );
        assert_eq!(
            resolve_bool_arg(&args, 0, &kwargs, "flag", "test_fn", false).unwrap(),
            false
        );
    }

    #[test]
    fn resolve_bool_arg_none_positional_returns_default() {
        let args = vec![MontyObject::None];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        assert_eq!(
            resolve_bool_arg(&args, 0, &kwargs, "flag", "test_fn", true).unwrap(),
            true
        );
    }

    #[test]
    fn resolve_bool_arg_wrong_type_returns_error() {
        let args = vec![MontyObject::Int(1)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let err = resolve_bool_arg(&args, 0, &kwargs, "flag", "fn_b", false).unwrap_err();
        assert!(is_type_error(&err));
        if let MontyObject::Exception { arg, .. } = &err {
            let msg = arg.as_ref().unwrap();
            assert!(msg.contains("fn_b"));
            assert!(msg.contains("'flag'"));
            assert!(msg.contains("bool"));
        }
    }

    // ── resolve_bytes_arg ───────────────────────────────────────────

    #[test]
    fn resolve_bytes_arg_from_positional() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let args = vec![MontyObject::Bytes(data.clone())];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_bytes_arg(&args, 0, &kwargs, "data", "test_fn").unwrap();
        assert_eq!(result, Some(&data));
    }

    #[test]
    fn resolve_bytes_arg_from_kwarg() {
        let data = vec![1, 2, 3];
        let args: Vec<MontyObject> = vec![];
        let kwargs = vec![kwarg("data", MontyObject::Bytes(data.clone()))];
        let result = resolve_bytes_arg(&args, 0, &kwargs, "data", "test_fn").unwrap();
        assert_eq!(result, Some(&data));
    }

    #[test]
    fn resolve_bytes_arg_absent() {
        let args: Vec<MontyObject> = vec![];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_bytes_arg(&args, 0, &kwargs, "data", "test_fn").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_bytes_arg_none_positional() {
        let args = vec![MontyObject::None];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_bytes_arg(&args, 0, &kwargs, "data", "test_fn").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_bytes_arg_wrong_type_returns_error() {
        let args = vec![s("not bytes")];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let err = resolve_bytes_arg(&args, 0, &kwargs, "data", "fn_c").unwrap_err();
        assert!(is_type_error(&err));
        if let MontyObject::Exception { arg, .. } = &err {
            let msg = arg.as_ref().unwrap();
            assert!(msg.contains("fn_c"));
            assert!(msg.contains("'data'"));
            assert!(msg.contains("bytes"));
        }
    }

    #[test]
    fn resolve_bytes_arg_empty_bytes() {
        let data: Vec<u8> = vec![];
        let args = vec![MontyObject::Bytes(data.clone())];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];
        let result = resolve_bytes_arg(&args, 0, &kwargs, "data", "test_fn").unwrap();
        assert_eq!(result, Some(&data));
    }

    // ── Multi-argument scenarios ────────────────────────────────────

    #[test]
    fn resolve_multiple_positional_args() {
        let args = vec![s("url_val"), s("GET"), MontyObject::Int(42)];
        let kwargs: Vec<(MontyObject, MontyObject)> = vec![];

        let r0 = resolve_str_arg(&args, 0, &kwargs, "url", "fn", None);
        assert_eq!(r0.unwrap(), Some("url_val".to_string()));

        let r1 = resolve_str_arg(&args, 1, &kwargs, "method", "fn", None);
        assert_eq!(r1.unwrap(), Some("GET".to_string()));

        let r2 = resolve_int_arg(&args, 2, &kwargs, "timeout", "fn", None);
        assert_eq!(r2.unwrap(), Some(42));
    }

    #[test]
    fn resolve_mixed_positional_and_kwargs() {
        let args = vec![s("positional_val")];
        let kwargs = vec![
            kwarg("second", MontyObject::Int(99)),
            kwarg("third", MontyObject::Bool(true)),
        ];

        let r0 = resolve_str_arg(&args, 0, &kwargs, "first", "fn", None);
        assert_eq!(r0.unwrap(), Some("positional_val".to_string()));

        let r1 = resolve_int_arg(&args, 1, &kwargs, "second", "fn", None);
        assert_eq!(r1.unwrap(), Some(99));

        let r2 = resolve_bool_arg(&args, 2, &kwargs, "third", "fn", false);
        assert_eq!(r2.unwrap(), true);
    }

    #[test]
    fn resolve_arg_skips_none_positional_to_kwarg() {
        // Simulates f(None, kwarg_b=42) where positional 0 is None
        let args = vec![MontyObject::None];
        let kwargs = vec![kwarg("a", MontyObject::Int(42))];
        let result = resolve_arg(&args, 0, &kwargs, "a");
        assert!(matches!(result, Some(MontyObject::Int(42))));
    }
}

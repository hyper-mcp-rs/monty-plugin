use monty::MontyObject;

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
            exc_type: monty::ExcType::TypeError,
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
            exc_type: monty::ExcType::TypeError,
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
            exc_type: monty::ExcType::TypeError,
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
            exc_type: monty::ExcType::TypeError,
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
            exc_type: monty::ExcType::TypeError,
            arg: Some(format!("{func_name}: '{name}' must be bytes")),
        }),
    }
}

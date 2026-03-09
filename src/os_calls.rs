use crate::python_args::{
    get_bool_kwarg, resolve_arg, resolve_bool_arg, resolve_bytes_arg, resolve_int_arg,
    resolve_str_arg,
};
use monty::{ExcType, MontyObject, OsFunction, dir_stat, file_stat, symlink_stat};
use std::path::Path;

/// Markdown description of the supported `pathlib.Path` and `os` operations.
pub(crate) const OS_CALLS_DESCRIPTION: &str = "\
Supported `pathlib.Path` operations:\n\
- `Path.exists(*, follow_symlinks: bool = True) -> bool`\n\
- `Path.is_file(*, follow_symlinks: bool = True) -> bool`\n\
- `Path.is_dir(*, follow_symlinks: bool = True) -> bool`\n\
- `Path.is_symlink() -> bool`\n\
- `Path.read_text(encoding: str | None = None, errors: str | None = None, newline: str | None = None) -> str`\n\
- `Path.read_bytes() -> bytes`\n\
- `Path.write_text(data: str, encoding: str | None = None, errors: str | None = None, newline: str | None = None) -> None`\n\
- `Path.write_bytes(data: bytes) -> None`\n\
- `Path.mkdir(mode=0o777, parents=False, exist_ok=False) -> None`\n\
- `Path.unlink(missing_ok=False) -> None`\n\
- `Path.rmdir() -> None`\n\
- `Path.iterdir() -> list[str]`\n\
- `Path.stat(*, follow_symlinks=True) -> os.stat_result`\n\
- `Path.rename(target) -> None`\n\
- `Path.resolve(strict=False) -> str`\n\
- `Path.absolute() -> str`\n\
\n\
Supported `os` operations:\n\
- `os.environ -> dict[str, str]`\n\
- `os.getenv(key: str, default=None) -> str | None`";

/// Handle OsCalls from the Monty VM.
pub(crate) fn handle_os_call(
    function: &OsFunction,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // Extract the path string from the first arg (if present) and slice
    // the remaining args so that callees index from 0.
    let (path_str, rest) = match args.first() {
        Some(MontyObject::Path(p)) => (Some(p.as_str()), &args[1..]),
        Some(MontyObject::String(s)) => (Some(s.as_str()), &args[1..]),
        _ => (None, args),
    };

    match function {
        OsFunction::Exists => path_exists(path_str, kwargs),
        OsFunction::IsFile => path_is_file(path_str, kwargs),
        OsFunction::IsDir => path_is_dir(path_str, kwargs),
        OsFunction::IsSymlink => path_is_symlink(path_str),
        OsFunction::ReadText => path_read_text(path_str, rest, kwargs),
        OsFunction::ReadBytes => path_read_bytes(path_str),
        OsFunction::WriteText => path_write_text(path_str, rest, kwargs),
        OsFunction::WriteBytes => path_write_bytes(path_str, rest, kwargs),
        OsFunction::Mkdir => path_mkdir(path_str, rest, kwargs),
        OsFunction::Unlink => path_unlink(path_str, rest, kwargs),
        OsFunction::Rmdir => path_rmdir(path_str),
        OsFunction::Iterdir => path_iterdir(path_str),
        OsFunction::Stat => path_stat(path_str, kwargs),
        OsFunction::Rename => path_rename(path_str, rest, kwargs),
        OsFunction::Resolve => path_resolve(path_str, rest, kwargs),
        OsFunction::Absolute => path_absolute(path_str),
        OsFunction::Getenv => os_getenv(args, kwargs),
        OsFunction::GetEnviron => os_get_environ(),
    }
}

// ---------------------------------------------------------------------------
// Path property checks
// ---------------------------------------------------------------------------

fn path_exists(path_str: Option<&str>, kwargs: &[(MontyObject, MontyObject)]) -> MontyObject {
    let follow_symlinks = match get_bool_kwarg(kwargs, "follow_symlinks", true, "Path.exists") {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Some(p) = path_str {
        let path = Path::new(p);
        if follow_symlinks {
            MontyObject::Bool(path.exists())
        } else {
            // Don't follow symlinks: use symlink_metadata which doesn't traverse.
            MontyObject::Bool(std::fs::symlink_metadata(path).is_ok())
        }
    } else {
        MontyObject::Bool(false)
    }
}

fn path_is_file(path_str: Option<&str>, kwargs: &[(MontyObject, MontyObject)]) -> MontyObject {
    let follow_symlinks = match get_bool_kwarg(kwargs, "follow_symlinks", true, "Path.is_file") {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Some(p) = path_str {
        let path = Path::new(p);
        if follow_symlinks {
            MontyObject::Bool(path.is_file())
        } else {
            // Don't follow symlinks: check symlink_metadata for file type.
            MontyObject::Bool(
                std::fs::symlink_metadata(path)
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false),
            )
        }
    } else {
        MontyObject::Bool(false)
    }
}

fn path_is_dir(path_str: Option<&str>, kwargs: &[(MontyObject, MontyObject)]) -> MontyObject {
    let follow_symlinks = match get_bool_kwarg(kwargs, "follow_symlinks", true, "Path.is_dir") {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Some(p) = path_str {
        let path = Path::new(p);
        if follow_symlinks {
            MontyObject::Bool(path.is_dir())
        } else {
            // Don't follow symlinks: check symlink_metadata for directory type.
            MontyObject::Bool(
                std::fs::symlink_metadata(path)
                    .map(|m| m.file_type().is_dir())
                    .unwrap_or(false),
            )
        }
    } else {
        MontyObject::Bool(false)
    }
}

fn path_is_symlink(path_str: Option<&str>) -> MontyObject {
    if let Some(p) = path_str {
        MontyObject::Bool(Path::new(p).is_symlink())
    } else {
        MontyObject::Bool(false)
    }
}

// ---------------------------------------------------------------------------
// Read file contents
// ---------------------------------------------------------------------------

fn path_read_text(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // Validate encoding — only UTF-8 (the default) is supported.
    match resolve_str_arg(args, 0, kwargs, "encoding", "Path.read_text", None) {
        Ok(Some(s)) if s.eq_ignore_ascii_case("utf-8") || s.eq_ignore_ascii_case("utf8") => {}
        Ok(Some(s)) => {
            return MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(format!(
                    "Path.read_text: unsupported encoding: '{s}' (only UTF-8 is supported)"
                )),
            };
        }
        Ok(None) => {}
        Err(e) => return e,
    }

    // Validate errors — only "strict" (the default) is supported.
    match resolve_str_arg(args, 1, kwargs, "errors", "Path.read_text", None) {
        Ok(Some(s)) if s == "strict" => {}
        Ok(Some(s)) => {
            return MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(format!(
                    "Path.read_text: unsupported error handler: '{s}' (only 'strict' is supported)"
                )),
            };
        }
        Ok(None) => {}
        Err(e) => return e,
    }

    // Validate newline — only None (universal newlines, the default) is supported.
    match resolve_str_arg(args, 2, kwargs, "newline", "Path.read_text", None) {
        Ok(Some(s)) => {
            return MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(format!(
                    "Path.read_text: unsupported newline mode: '{s}' (only None is supported)"
                )),
            };
        }
        Ok(None) => {}
        Err(e) => return e,
    }

    if let Some(p) = path_str {
        match std::fs::read_to_string(p) {
            Ok(contents) => MontyObject::String(contents),
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some("read_text: no path provided".into()),
        }
    }
}

fn path_read_bytes(path_str: Option<&str>) -> MontyObject {
    if let Some(p) = path_str {
        match std::fs::read(p) {
            Ok(bytes) => MontyObject::Bytes(bytes),
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some("read_bytes: no path provided".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

fn path_write_text(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // data (required, positional-or-keyword at index 0)
    let data = match resolve_str_arg(args, 0, kwargs, "data", "Path.write_text", None) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("Path.write_text: missing required argument: 'data'".into()),
            };
        }
        Err(e) => return e,
    };

    // Validate encoding — only UTF-8 (the default) is supported.
    match resolve_str_arg(args, 1, kwargs, "encoding", "Path.write_text", None) {
        Ok(Some(s)) if s.eq_ignore_ascii_case("utf-8") || s.eq_ignore_ascii_case("utf8") => {}
        Ok(Some(s)) => {
            return MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(format!(
                    "Path.write_text: unsupported encoding: '{s}' (only UTF-8 is supported)"
                )),
            };
        }
        Ok(None) => {}
        Err(e) => return e,
    }

    // Validate errors — only "strict" (the default) is supported.
    match resolve_str_arg(args, 2, kwargs, "errors", "Path.write_text", None) {
        Ok(Some(s)) if s == "strict" => {}
        Ok(Some(s)) => {
            return MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(format!(
                    "Path.write_text: unsupported error handler: '{s}' (only 'strict' is supported)"
                )),
            };
        }
        Ok(None) => {}
        Err(e) => return e,
    }

    // Validate newline — only None (universal newlines, the default) is supported.
    match resolve_str_arg(args, 3, kwargs, "newline", "Path.write_text", None) {
        Ok(Some(s)) => {
            return MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(format!(
                    "Path.write_text: unsupported newline mode: '{s}' (only None is supported)"
                )),
            };
        }
        Ok(None) => {}
        Err(e) => return e,
    }

    if let Some(p) = path_str {
        match std::fs::write(p, data) {
            Ok(()) => MontyObject::None,
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some("write_text: no path provided".into()),
        }
    }
}

fn path_write_bytes(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // data (required, positional-or-keyword at index 0)
    let data = match resolve_bytes_arg(args, 0, kwargs, "data", "Path.write_bytes") {
        Ok(Some(b)) => b,
        Ok(None) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("Path.write_bytes: missing required argument: 'data'".into()),
            };
        }
        Err(e) => return e,
    };

    if let Some(p) = path_str {
        match std::fs::write(p, data) {
            Ok(()) => MontyObject::None,
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some("write_bytes: no path provided".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Directory operations
// ---------------------------------------------------------------------------

fn path_mkdir(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // mode (index 0) — accept an int but ignore it (no portable permission
    // support in WASM); default 0o777.
    match resolve_int_arg(args, 0, kwargs, "mode", "Path.mkdir", Some(0o777)) {
        Ok(_) => {}
        Err(e) => return e,
    }

    // parents (index 1) — default False.
    let parents = match resolve_bool_arg(args, 1, kwargs, "parents", "Path.mkdir", false) {
        Ok(v) => v,
        Err(e) => return e,
    };

    // exist_ok (index 2) — default False.
    let exist_ok = match resolve_bool_arg(args, 2, kwargs, "exist_ok", "Path.mkdir", false) {
        Ok(v) => v,
        Err(e) => return e,
    };

    if let Some(p) = path_str {
        let result = if parents {
            std::fs::create_dir_all(p)
        } else {
            std::fs::create_dir(p)
        };
        match result {
            Ok(()) => MontyObject::None,
            Err(e) if exist_ok && e.kind() == std::io::ErrorKind::AlreadyExists => {
                MontyObject::None
            }
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::None
    }
}

fn path_unlink(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // missing_ok (index 0) — default False.
    let missing_ok = match resolve_bool_arg(args, 0, kwargs, "missing_ok", "Path.unlink", false) {
        Ok(v) => v,
        Err(e) => return e,
    };

    if let Some(p) = path_str {
        match std::fs::remove_file(p) {
            Ok(()) => MontyObject::None,
            Err(e) if missing_ok && e.kind() == std::io::ErrorKind::NotFound => MontyObject::None,
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::None
    }
}

fn path_rmdir(path_str: Option<&str>) -> MontyObject {
    if let Some(p) = path_str {
        match std::fs::remove_dir(p) {
            Ok(()) => MontyObject::None,
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::None
    }
}

fn path_iterdir(path_str: Option<&str>) -> MontyObject {
    if let Some(p) = path_str {
        match std::fs::read_dir(p) {
            Ok(entries) => {
                let items: Vec<MontyObject> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| MontyObject::String(e.path().to_string_lossy().into_owned()))
                    .collect();
                MontyObject::List(items)
            }
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::List(vec![])
    }
}

// ---------------------------------------------------------------------------
// Stat
// ---------------------------------------------------------------------------

fn path_stat(path_str: Option<&str>, kwargs: &[(MontyObject, MontyObject)]) -> MontyObject {
    let follow_symlinks = match get_bool_kwarg(kwargs, "follow_symlinks", true, "Path.stat") {
        Ok(v) => v,
        Err(e) => return e,
    };

    if let Some(p) = path_str {
        let result = if follow_symlinks {
            std::fs::metadata(p)
        } else {
            std::fs::symlink_metadata(p)
        };
        match result {
            Ok(meta) => {
                let size = meta.len() as i64;
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                if meta.is_dir() {
                    dir_stat(0o755, mtime)
                } else if meta.is_symlink() {
                    symlink_stat(0o777, mtime)
                } else {
                    file_stat(0o644, size, mtime)
                }
            }
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::None
    }
}

// ---------------------------------------------------------------------------
// Rename
// ---------------------------------------------------------------------------

fn path_rename(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // target (required, positional-or-keyword at index 0)
    let dest = match resolve_arg(args, 0, kwargs, "target") {
        Some(MontyObject::Path(p)) => p.as_str(),
        Some(MontyObject::String(s)) => s.as_str(),
        Some(_) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("Path.rename: 'target' must be a string or Path".into()),
            };
        }
        None => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("Path.rename: missing required argument: 'target'".into()),
            };
        }
    };

    if let Some(src) = path_str {
        match std::fs::rename(src, dest) {
            Ok(()) => MontyObject::None,
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("{e}")),
            },
        }
    } else {
        MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some("rename: no path provided".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Resolve / Absolute
// ---------------------------------------------------------------------------

fn path_resolve(
    path_str: Option<&str>,
    args: &[MontyObject],
    kwargs: &[(MontyObject, MontyObject)],
) -> MontyObject {
    // strict (index 0) — default False.
    let strict = match resolve_bool_arg(args, 0, kwargs, "strict", "Path.resolve", false) {
        Ok(v) => v,
        Err(e) => return e,
    };

    if let Some(p) = path_str {
        if strict {
            // strict=True: path must exist, use canonicalize which resolves
            // symlinks and errors if the path doesn't exist.
            match std::fs::canonicalize(p) {
                Ok(resolved) => MontyObject::String(resolved.to_string_lossy().into_owned()),
                Err(e) => MontyObject::Exception {
                    exc_type: ExcType::OSError,
                    arg: Some(format!("{e}")),
                },
            }
        } else {
            // strict=False: resolve what we can without requiring existence.
            let path = Path::new(p);
            MontyObject::String(
                std::env::current_dir()
                    .map(|cwd| {
                        let abs = if path.is_absolute() {
                            path.to_path_buf()
                        } else {
                            cwd.join(path)
                        };
                        // Attempt canonicalize; fall back to the joined path.
                        std::fs::canonicalize(&abs)
                            .unwrap_or(abs)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|_| p.to_string()),
            )
        }
    } else {
        MontyObject::String(String::new())
    }
}

fn path_absolute(path_str: Option<&str>) -> MontyObject {
    if let Some(p) = path_str {
        let abs = Path::new(p);
        MontyObject::String(
            std::env::current_dir()
                .map(|cwd| cwd.join(abs).to_string_lossy().into_owned())
                .unwrap_or_else(|_| p.to_string()),
        )
    } else {
        MontyObject::String(String::new())
    }
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

fn os_getenv(args: &[MontyObject], kwargs: &[(MontyObject, MontyObject)]) -> MontyObject {
    // key (required, positional-or-keyword at index 0)
    let key = match resolve_str_arg(args, 0, kwargs, "key", "os.getenv", None) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("os.getenv: missing required argument: 'key'".into()),
            };
        }
        Err(e) => return e,
    };

    // default (optional, positional-or-keyword at index 1) — default None.
    let default = match resolve_arg(args, 1, kwargs, "default") {
        Some(v) => v.clone(),
        None => MontyObject::None,
    };

    match std::env::var(&key) {
        Ok(val) => MontyObject::String(val),
        Err(_) => default,
    }
}

fn os_get_environ() -> MontyObject {
    let pairs: Vec<(MontyObject, MontyObject)> = std::env::vars()
        .map(|(k, v)| (MontyObject::String(k), MontyObject::String(v)))
        .collect();
    MontyObject::Dict(pairs.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use monty::{ExcType, MontyObject, OsFunction};
    use std::io::Write;
    use tempfile::TempDir;

    // ── Helpers ──────────────────────────────────────────────────────

    fn kwarg(name: &str, val: MontyObject) -> (MontyObject, MontyObject) {
        (MontyObject::String(name.to_string()), val)
    }

    fn s(val: &str) -> MontyObject {
        MontyObject::String(val.to_string())
    }

    fn no_kwargs() -> Vec<(MontyObject, MontyObject)> {
        vec![]
    }

    fn is_bool(obj: &MontyObject, expected: bool) -> bool {
        matches!(obj, MontyObject::Bool(b) if *b == expected)
    }

    fn is_exception_of(obj: &MontyObject, expected: ExcType) -> bool {
        matches!(obj, MontyObject::Exception { exc_type, .. } if *exc_type == expected)
    }

    /// Create a temp file inside `dir` with the given name and content.
    fn create_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path.to_string_lossy().into_owned()
    }

    /// Return the string path for a child inside a TempDir (doesn't create it).
    fn child_path(dir: &TempDir, name: &str) -> String {
        dir.path().join(name).to_string_lossy().into_owned()
    }

    // ── handle_os_call dispatch ─────────────────────────────────────

    #[test]
    fn handle_os_call_dispatches_with_path_arg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hello");
        let args = vec![MontyObject::Path(p)];
        let result = handle_os_call(&OsFunction::Exists, &args, &no_kwargs());
        assert!(is_bool(&result, true));
    }

    #[test]
    fn handle_os_call_dispatches_with_string_arg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hello");
        let args = vec![s(&p)];
        let result = handle_os_call(&OsFunction::Exists, &args, &no_kwargs());
        assert!(is_bool(&result, true));
    }

    #[test]
    fn handle_os_call_dispatches_getenv() {
        // Getenv receives the raw args (no path extraction).
        let args = vec![s("PATH")];
        let result = handle_os_call(&OsFunction::Getenv, &args, &no_kwargs());
        assert!(matches!(result, MontyObject::String(_)));
    }

    #[test]
    fn handle_os_call_dispatches_get_environ() {
        let result = handle_os_call(&OsFunction::GetEnviron, &[], &no_kwargs());
        assert!(matches!(result, MontyObject::Dict(_)));
    }

    // ── path_exists ─────────────────────────────────────────────────

    #[test]
    fn path_exists_true_for_existing_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "exists.txt", "hi");
        assert!(is_bool(&path_exists(Some(&p), &no_kwargs()), true));
    }

    #[test]
    fn path_exists_false_for_nonexistent() {
        assert!(is_bool(
            &path_exists(Some("/tmp/__no_such_file__"), &no_kwargs()),
            false
        ));
    }

    #[test]
    fn path_exists_false_when_no_path() {
        assert!(is_bool(&path_exists(None, &no_kwargs()), false));
    }

    #[test]
    fn path_exists_follow_symlinks_kwarg_error_on_wrong_type() {
        let kwargs = vec![kwarg("follow_symlinks", s("yes"))];
        let result = path_exists(Some("/tmp"), &kwargs);
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_exists_true_for_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        assert!(is_bool(&path_exists(Some(&p), &no_kwargs()), true));
    }

    // ── path_is_file ────────────────────────────────────────────────

    #[test]
    fn path_is_file_true_for_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "data");
        assert!(is_bool(&path_is_file(Some(&p), &no_kwargs()), true));
    }

    #[test]
    fn path_is_file_false_for_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        assert!(is_bool(&path_is_file(Some(&p), &no_kwargs()), false));
    }

    #[test]
    fn path_is_file_false_for_nonexistent() {
        assert!(is_bool(
            &path_is_file(Some("/tmp/__nope__"), &no_kwargs()),
            false
        ));
    }

    #[test]
    fn path_is_file_false_when_no_path() {
        assert!(is_bool(&path_is_file(None, &no_kwargs()), false));
    }

    #[test]
    fn path_is_file_follow_symlinks_kwarg_error() {
        let kwargs = vec![kwarg("follow_symlinks", MontyObject::Int(1))];
        let result = path_is_file(Some("/tmp"), &kwargs);
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    // ── path_is_dir ─────────────────────────────────────────────────

    #[test]
    fn path_is_dir_true_for_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        assert!(is_bool(&path_is_dir(Some(&p), &no_kwargs()), true));
    }

    #[test]
    fn path_is_dir_false_for_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "x");
        assert!(is_bool(&path_is_dir(Some(&p), &no_kwargs()), false));
    }

    #[test]
    fn path_is_dir_false_for_nonexistent() {
        assert!(is_bool(
            &path_is_dir(Some("/tmp/__nope__"), &no_kwargs()),
            false
        ));
    }

    #[test]
    fn path_is_dir_false_when_no_path() {
        assert!(is_bool(&path_is_dir(None, &no_kwargs()), false));
    }

    // ── path_is_symlink ─────────────────────────────────────────────

    #[test]
    fn path_is_symlink_false_for_regular_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "x");
        assert!(is_bool(&path_is_symlink(Some(&p)), false));
    }

    #[test]
    fn path_is_symlink_false_when_no_path() {
        assert!(is_bool(&path_is_symlink(None), false));
    }

    #[cfg(unix)]
    #[test]
    fn path_is_symlink_true_for_symlink() {
        let dir = TempDir::new().unwrap();
        let target = create_file(&dir, "target.txt", "real");
        let link = child_path(&dir, "link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(is_bool(&path_is_symlink(Some(&link)), true));
    }

    // ── path_read_text ──────────────────────────────────────────────

    #[test]
    fn path_read_text_success() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "hello.txt", "hello world");
        let result = path_read_text(Some(&p), &[], &no_kwargs());
        assert!(matches!(result, MontyObject::String(ref s) if s == "hello world"));
    }

    #[test]
    fn path_read_text_nonexistent_file() {
        let result = path_read_text(Some("/tmp/__no_such_file_read__"), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_read_text_no_path() {
        let result = path_read_text(None, &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("no path"));
        }
    }

    #[test]
    fn path_read_text_utf8_encoding_accepted() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "café");
        let args = vec![s("utf-8")];
        let result = path_read_text(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::String(ref s) if s == "café"));
    }

    #[test]
    fn path_read_text_utf8_encoding_case_insensitive() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "ok");
        let args = vec![s("UTF-8")];
        let result = path_read_text(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::String(_)));
    }

    #[test]
    fn path_read_text_unsupported_encoding() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let args = vec![s("latin-1")];
        let result = path_read_text(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::ValueError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("latin-1"));
        }
    }

    #[test]
    fn path_read_text_unsupported_errors_handler() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let args = vec![MontyObject::None, s("ignore")];
        let result = path_read_text(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::ValueError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("ignore"));
        }
    }

    #[test]
    fn path_read_text_strict_errors_accepted() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "ok");
        let args = vec![MontyObject::None, s("strict")];
        let result = path_read_text(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::String(ref s) if s == "ok"));
    }

    #[test]
    fn path_read_text_unsupported_newline() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let args = vec![MontyObject::None, MontyObject::None, s("\r\n")];
        let result = path_read_text(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_read_text_encoding_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "café");
        let kwargs = vec![kwarg("encoding", s("utf-8"))];
        let result = path_read_text(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::String(ref s) if s == "café"));
    }

    #[test]
    fn path_read_text_unsupported_encoding_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let kwargs = vec![kwarg("encoding", s("latin-1"))];
        let result = path_read_text(Some(&p), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_read_text_errors_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "ok");
        let kwargs = vec![kwarg("errors", s("strict"))];
        let result = path_read_text(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::String(ref s) if s == "ok"));
    }

    #[test]
    fn path_read_text_unsupported_errors_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let kwargs = vec![kwarg("errors", s("ignore"))];
        let result = path_read_text(Some(&p), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_read_text_unsupported_newline_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let kwargs = vec![kwarg("newline", s("\r\n"))];
        let result = path_read_text(Some(&p), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    // ── path_read_bytes ─────────────────────────────────────────────

    #[test]
    fn path_read_bytes_success() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "data.bin", "binary");
        let result = path_read_bytes(Some(&p));
        assert!(matches!(result, MontyObject::Bytes(ref b) if b == b"binary"));
    }

    #[test]
    fn path_read_bytes_nonexistent() {
        let result = path_read_bytes(Some("/tmp/__no_such_file_bytes__"));
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_read_bytes_no_path() {
        let result = path_read_bytes(None);
        assert!(is_exception_of(&result, ExcType::OSError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("no path"));
        }
    }

    // ── path_write_text ─────────────────────────────────────────────

    #[test]
    fn path_write_text_success() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let args = vec![s("written content")];
        let result = path_write_text(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "written content");
    }

    #[test]
    fn path_write_text_missing_data() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let result = path_write_text(Some(&p), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("data"));
        }
    }

    #[test]
    fn path_write_text_no_path() {
        let args = vec![s("data")];
        let result = path_write_text(None, &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_write_text_unsupported_encoding() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let args = vec![s("data"), s("ascii")];
        let result = path_write_text(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::ValueError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("ascii"));
        }
    }

    #[test]
    fn path_write_text_utf8_encoding_accepted() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let args = vec![s("data"), s("utf-8")];
        let result = path_write_text(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_write_text_data_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let kwargs = vec![kwarg("data", s("kwarg content"))];
        let result = path_write_text(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "kwarg content");
    }

    #[test]
    fn path_write_text_encoding_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let kwargs = vec![kwarg("data", s("hi")), kwarg("encoding", s("utf-8"))];
        let result = path_write_text(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_write_text_unsupported_encoding_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let kwargs = vec![kwarg("data", s("hi")), kwarg("encoding", s("ascii"))];
        let result = path_write_text(Some(&p), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_write_text_errors_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let kwargs = vec![kwarg("data", s("hi")), kwarg("errors", s("replace"))];
        let result = path_write_text(Some(&p), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_write_text_newline_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let kwargs = vec![kwarg("data", s("hi")), kwarg("newline", s("\n"))];
        let result = path_write_text(Some(&p), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_write_text_unsupported_errors_handler() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let args = vec![s("data"), MontyObject::None, s("replace")];
        let result = path_write_text(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    #[test]
    fn path_write_text_unsupported_newline() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let args = vec![s("data"), MontyObject::None, MontyObject::None, s("\n")];
        let result = path_write_text(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::ValueError));
    }

    // ── path_write_bytes ────────────────────────────────────────────

    #[test]
    fn path_write_bytes_success() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.bin");
        let args = vec![MontyObject::Bytes(vec![1, 2, 3])];
        let result = path_write_bytes(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert_eq!(std::fs::read(&p).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn path_write_bytes_missing_data() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.bin");
        let result = path_write_bytes(Some(&p), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_write_bytes_wrong_type() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.bin");
        let args = vec![s("not bytes")];
        let result = path_write_bytes(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_write_bytes_no_path() {
        let args = vec![MontyObject::Bytes(vec![1])];
        let result = path_write_bytes(None, &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_write_bytes_data_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.bin");
        let kwargs = vec![kwarg("data", MontyObject::Bytes(vec![4, 5, 6]))];
        let result = path_write_bytes(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
        assert_eq!(std::fs::read(&p).unwrap(), vec![4, 5, 6]);
    }

    // ── path_mkdir ──────────────────────────────────────────────────

    #[test]
    fn path_mkdir_creates_directory() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "newdir");
        let result = path_mkdir(Some(&p), &[], &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert!(Path::new(&p).is_dir());
    }

    #[test]
    fn path_mkdir_fails_without_parents() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "a/b/c");
        let result = path_mkdir(Some(&p), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_mkdir_with_parents() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "a/b/c");
        // mode=None, parents=True
        let args = vec![MontyObject::None, MontyObject::Bool(true)];
        let result = path_mkdir(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert!(Path::new(&p).is_dir());
    }

    #[test]
    fn path_mkdir_exist_ok() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "existing");
        std::fs::create_dir(&p).unwrap();
        // mode=None, parents=False, exist_ok=True
        let args = vec![
            MontyObject::None,
            MontyObject::Bool(false),
            MontyObject::Bool(true),
        ];
        let result = path_mkdir(Some(&p), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_mkdir_already_exists_no_exist_ok() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "existing");
        std::fs::create_dir(&p).unwrap();
        let result = path_mkdir(Some(&p), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_mkdir_no_path_returns_none() {
        let result = path_mkdir(None, &[], &no_kwargs());
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_mkdir_mode_wrong_type() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "newdir");
        let args = vec![s("bad_mode")];
        let result = path_mkdir(Some(&p), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_mkdir_parents_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "a/b/c");
        let kwargs = vec![kwarg("parents", MontyObject::Bool(true))];
        let result = path_mkdir(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
        assert!(Path::new(&p).is_dir());
    }

    #[test]
    fn path_mkdir_exist_ok_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "existing");
        std::fs::create_dir(&p).unwrap();
        let kwargs = vec![kwarg("exist_ok", MontyObject::Bool(true))];
        let result = path_mkdir(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_mkdir_mode_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "modedir");
        let kwargs = vec![kwarg("mode", MontyObject::Int(0o755))];
        let result = path_mkdir(Some(&p), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
        assert!(Path::new(&p).is_dir());
    }

    // ── path_unlink ─────────────────────────────────────────────────

    #[test]
    fn path_unlink_removes_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "to_delete.txt", "bye");
        let result = path_unlink(Some(&p), &[], &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert!(!Path::new(&p).exists());
    }

    #[test]
    fn path_unlink_nonexistent_fails() {
        let result = path_unlink(Some("/tmp/__no_such_file_unlink__"), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_unlink_missing_ok() {
        let result = path_unlink(
            Some("/tmp/__no_such_file_unlink_ok__"),
            &[MontyObject::Bool(true)],
            &no_kwargs(),
        );
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_unlink_no_path_returns_none() {
        let result = path_unlink(None, &[], &no_kwargs());
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_unlink_missing_ok_wrong_type() {
        let args = vec![s("yes")];
        let result = path_unlink(Some("/tmp/__any__"), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_unlink_missing_ok_via_kwarg() {
        let kwargs = vec![kwarg("missing_ok", MontyObject::Bool(true))];
        let result = path_unlink(Some("/tmp/__no_such_unlink_kwarg__"), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
    }

    // ── path_rmdir ──────────────────────────────────────────────────

    #[test]
    fn path_rmdir_removes_empty_dir() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "empty");
        std::fs::create_dir(&p).unwrap();
        let result = path_rmdir(Some(&p));
        assert!(matches!(result, MontyObject::None));
        assert!(!Path::new(&p).exists());
    }

    #[test]
    fn path_rmdir_fails_on_nonempty_dir() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "nonempty");
        std::fs::create_dir(&p).unwrap();
        std::fs::write(Path::new(&p).join("child.txt"), "x").unwrap();
        let result = path_rmdir(Some(&p));
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_rmdir_nonexistent() {
        let result = path_rmdir(Some("/tmp/__no_such_dir_rmdir__"));
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_rmdir_no_path_returns_none() {
        let result = path_rmdir(None);
        assert!(matches!(result, MontyObject::None));
    }

    // ── path_iterdir ────────────────────────────────────────────────

    #[test]
    fn path_iterdir_lists_contents() {
        let dir = TempDir::new().unwrap();
        create_file(&dir, "a.txt", "a");
        create_file(&dir, "b.txt", "b");
        let p = dir.path().to_string_lossy().into_owned();
        let result = path_iterdir(Some(&p));
        if let MontyObject::List(items) = &result {
            assert_eq!(items.len(), 2);
            // All items should be strings
            for item in items {
                assert!(matches!(item, MontyObject::String(_)));
            }
        } else {
            panic!("expected List, got {:?}", result);
        }
    }

    #[test]
    fn path_iterdir_empty_dir() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        let result = path_iterdir(Some(&p));
        assert!(matches!(result, MontyObject::List(ref v) if v.is_empty()));
    }

    #[test]
    fn path_iterdir_nonexistent() {
        let result = path_iterdir(Some("/tmp/__no_such_dir_iterdir__"));
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_iterdir_no_path_returns_empty_list() {
        let result = path_iterdir(None);
        assert!(matches!(result, MontyObject::List(ref v) if v.is_empty()));
    }

    // ── path_stat ───────────────────────────────────────────────────

    #[test]
    fn path_stat_file_returns_named_tuple() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "stat_me.txt", "some data");
        let result = path_stat(Some(&p), &no_kwargs());
        assert!(matches!(result, MontyObject::NamedTuple { .. }));
    }

    #[test]
    fn path_stat_directory_returns_named_tuple() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        let result = path_stat(Some(&p), &no_kwargs());
        assert!(matches!(result, MontyObject::NamedTuple { .. }));
    }

    #[test]
    fn path_stat_nonexistent() {
        let result = path_stat(Some("/tmp/__no_such_file_stat__"), &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_stat_no_path_returns_none() {
        let result = path_stat(None, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn path_stat_follow_symlinks_kwarg_wrong_type() {
        let kwargs = vec![kwarg("follow_symlinks", s("yes"))];
        let result = path_stat(Some("/tmp"), &kwargs);
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    // ── path_rename ─────────────────────────────────────────────────

    #[test]
    fn path_rename_success() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "old.txt", "content");
        let dst = child_path(&dir, "new.txt");
        let args = vec![s(&dst)];
        let result = path_rename(Some(&src), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert!(!Path::new(&src).exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "content");
    }

    #[test]
    fn path_rename_with_path_target() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "old.txt", "data");
        let dst = child_path(&dir, "new.txt");
        let args = vec![MontyObject::Path(dst.clone())];
        let result = path_rename(Some(&src), &args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "data");
    }

    #[test]
    fn path_rename_missing_target() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "f.txt", "x");
        let result = path_rename(Some(&src), &[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("target"));
        }
    }

    #[test]
    fn path_rename_target_wrong_type() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "f.txt", "x");
        let args = vec![MontyObject::Int(42)];
        let result = path_rename(Some(&src), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_rename_no_path() {
        let dir = TempDir::new().unwrap();
        let dst = child_path(&dir, "dst.txt");
        let args = vec![s(&dst)];
        let result = path_rename(None, &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("no path"));
        }
    }

    #[test]
    fn path_rename_nonexistent_source() {
        let dir = TempDir::new().unwrap();
        let dst = child_path(&dir, "dst.txt");
        let args = vec![s(&dst)];
        let result = path_rename(Some("/tmp/__no_such_rename_src__"), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_rename_target_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "old.txt", "kwarg rename");
        let dst = child_path(&dir, "new.txt");
        let kwargs = vec![kwarg("target", s(&dst))];
        let result = path_rename(Some(&src), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
        assert!(!Path::new(&src).exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "kwarg rename");
    }

    #[test]
    fn path_rename_target_path_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "old2.txt", "path kwarg");
        let dst = child_path(&dir, "new2.txt");
        let kwargs = vec![kwarg("target", MontyObject::Path(dst.clone()))];
        let result = path_rename(Some(&src), &[], &kwargs);
        assert!(matches!(result, MontyObject::None));
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "path kwarg");
    }

    // ── path_resolve ────────────────────────────────────────────────

    #[test]
    fn path_resolve_existing_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "resolve_me.txt", "ok");
        let result = path_resolve(Some(&p), &[], &no_kwargs());
        if let MontyObject::String(resolved) = &result {
            assert!(Path::new(resolved).is_absolute());
        } else {
            panic!("expected String, got {:?}", result);
        }
    }

    #[test]
    fn path_resolve_strict_nonexistent_fails() {
        let args = vec![MontyObject::Bool(true)];
        let result = path_resolve(Some("/tmp/__no_such_resolve_strict__"), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    #[test]
    fn path_resolve_non_strict_nonexistent_returns_string() {
        let result = path_resolve(Some("relative/nonexistent"), &[], &no_kwargs());
        assert!(matches!(result, MontyObject::String(_)));
    }

    #[test]
    fn path_resolve_no_path_returns_empty_string() {
        let result = path_resolve(None, &[], &no_kwargs());
        assert!(matches!(result, MontyObject::String(ref s) if s.is_empty()));
    }

    #[test]
    fn path_resolve_strict_kwarg_wrong_type() {
        let args = vec![s("yes")];
        let result = path_resolve(Some("/tmp"), &args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn path_resolve_strict_existing_returns_absolute() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "strict.txt", "ok");
        let args = vec![MontyObject::Bool(true)];
        let result = path_resolve(Some(&p), &args, &no_kwargs());
        if let MontyObject::String(resolved) = &result {
            assert!(Path::new(resolved).is_absolute());
        } else {
            panic!("expected String, got {:?}", result);
        }
    }

    #[test]
    fn path_resolve_strict_via_kwarg() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "strict_kw.txt", "ok");
        let kwargs = vec![kwarg("strict", MontyObject::Bool(true))];
        let result = path_resolve(Some(&p), &[], &kwargs);
        if let MontyObject::String(resolved) = &result {
            assert!(Path::new(resolved).is_absolute());
        } else {
            panic!("expected String, got {:?}", result);
        }
    }

    #[test]
    fn path_resolve_strict_nonexistent_via_kwarg() {
        let kwargs = vec![kwarg("strict", MontyObject::Bool(true))];
        let result = path_resolve(Some("/tmp/__no_such_resolve_kw__"), &[], &kwargs);
        assert!(is_exception_of(&result, ExcType::OSError));
    }

    // ── path_absolute ───────────────────────────────────────────────

    #[test]
    fn path_absolute_returns_absolute_path() {
        let result = path_absolute(Some("some/relative"));
        if let MontyObject::String(abs) = &result {
            assert!(Path::new(abs).is_absolute());
            assert!(abs.ends_with("some/relative"));
        } else {
            panic!("expected String, got {:?}", result);
        }
    }

    #[test]
    fn path_absolute_already_absolute() {
        let result = path_absolute(Some("/already/absolute"));
        if let MontyObject::String(abs) = &result {
            assert!(abs.contains("/already/absolute"));
        } else {
            panic!("expected String, got {:?}", result);
        }
    }

    #[test]
    fn path_absolute_no_path_returns_empty_string() {
        let result = path_absolute(None);
        assert!(matches!(result, MontyObject::String(ref s) if s.is_empty()));
    }

    // ── os_getenv ───────────────────────────────────────────────────

    #[test]
    fn os_getenv_existing_var() {
        // PATH should exist on all platforms
        let args = vec![s("PATH")];
        let result = os_getenv(&args, &no_kwargs());
        assert!(matches!(result, MontyObject::String(_)));
    }

    #[test]
    fn os_getenv_nonexistent_returns_none() {
        let args = vec![s("__MONTY_PLUGIN_TEST_NONEXISTENT_VAR__")];
        let result = os_getenv(&args, &no_kwargs());
        assert!(matches!(result, MontyObject::None));
    }

    #[test]
    fn os_getenv_nonexistent_with_default() {
        let args = vec![s("__MONTY_PLUGIN_TEST_NONEXISTENT_VAR__"), s("fallback")];
        let result = os_getenv(&args, &no_kwargs());
        assert!(matches!(result, MontyObject::String(ref s) if s == "fallback"));
    }

    #[test]
    fn os_getenv_nonexistent_with_int_default() {
        let args = vec![
            s("__MONTY_PLUGIN_TEST_NONEXISTENT_VAR__"),
            MontyObject::Int(42),
        ];
        let result = os_getenv(&args, &no_kwargs());
        assert!(matches!(result, MontyObject::Int(42)));
    }

    #[test]
    fn os_getenv_missing_key() {
        let result = os_getenv(&[], &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
        if let MontyObject::Exception { arg, .. } = &result {
            assert!(arg.as_ref().unwrap().contains("key"));
        }
    }

    #[test]
    fn os_getenv_key_wrong_type() {
        let args = vec![MontyObject::Int(123)];
        let result = os_getenv(&args, &no_kwargs());
        assert!(is_exception_of(&result, ExcType::TypeError));
    }

    #[test]
    fn os_getenv_via_kwargs() {
        let kwargs = vec![kwarg("key", s("PATH"))];
        let result = os_getenv(&[], &kwargs);
        assert!(matches!(result, MontyObject::String(_)));
    }

    #[test]
    fn os_getenv_default_via_kwarg() {
        let kwargs = vec![
            kwarg("key", s("__MONTY_PLUGIN_TEST_NONEXISTENT_VAR__")),
            kwarg("default", s("kwarg_fallback")),
        ];
        let result = os_getenv(&[], &kwargs);
        assert!(matches!(result, MontyObject::String(ref s) if s == "kwarg_fallback"));
    }

    // ── os_get_environ ──────────────────────────────────────────────

    #[test]
    fn os_get_environ_returns_dict() {
        let result = os_get_environ();
        assert!(matches!(result, MontyObject::Dict(_)));
    }

    #[test]
    fn os_get_environ_contains_path() {
        let result = os_get_environ();
        // Serialize to JSON and check for "PATH" key in the dict pairs
        let json = serde_json::to_value(&result).unwrap();
        let pairs = json.get("Dict").unwrap().as_array().unwrap();
        let has_path = pairs.iter().any(|pair| {
            pair.as_array()
                .and_then(|a| a.first())
                .and_then(|k| k.get("String"))
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "PATH")
        });
        assert!(has_path, "expected PATH in environ");
    }

    #[test]
    fn os_get_environ_all_entries_are_string_pairs() {
        let result = os_get_environ();
        let json = serde_json::to_value(&result).unwrap();
        let pairs = json.get("Dict").unwrap().as_array().unwrap();
        for pair in pairs {
            let arr = pair.as_array().unwrap();
            assert_eq!(arr.len(), 2);
            assert!(arr[0].get("String").is_some(), "key should be String");
            assert!(arr[1].get("String").is_some(), "value should be String");
        }
    }

    // ── write then read round-trips ─────────────────────────────────

    #[test]
    fn write_text_then_read_text_roundtrip() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "roundtrip.txt");
        let content = "hello\nworld\n🎉";
        let w_args = vec![s(content)];
        let w = path_write_text(Some(&p), &w_args, &no_kwargs());
        assert!(matches!(w, MontyObject::None));
        let r = path_read_text(Some(&p), &[], &no_kwargs());
        assert!(matches!(r, MontyObject::String(ref s) if s == content));
    }

    #[test]
    fn write_bytes_then_read_bytes_roundtrip() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "roundtrip.bin");
        let data = vec![0, 1, 127, 128, 255];
        let w_args = vec![MontyObject::Bytes(data.clone())];
        let w = path_write_bytes(Some(&p), &w_args, &no_kwargs());
        assert!(matches!(w, MontyObject::None));
        let r = path_read_bytes(Some(&p));
        assert!(matches!(r, MontyObject::Bytes(ref b) if *b == data));
    }

    // ── mkdir then iterdir then rmdir round-trip ────────────────────

    #[test]
    fn mkdir_iterdir_rmdir_lifecycle() {
        let dir = TempDir::new().unwrap();
        let sub = child_path(&dir, "sub");

        // mkdir
        let result = path_mkdir(Some(&sub), &[], &no_kwargs());
        assert!(matches!(result, MontyObject::None));

        // iterdir on parent shows our sub
        let parent = dir.path().to_string_lossy().into_owned();
        let list = path_iterdir(Some(&parent));
        if let MontyObject::List(items) = &list {
            assert_eq!(items.len(), 1);
        } else {
            panic!("expected List");
        }

        // rmdir
        let result = path_rmdir(Some(&sub));
        assert!(matches!(result, MontyObject::None));
        assert!(!Path::new(&sub).exists());
    }
}

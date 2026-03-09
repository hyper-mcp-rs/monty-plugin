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

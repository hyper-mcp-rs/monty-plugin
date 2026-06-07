use chrono::{Datelike, FixedOffset, Local, Timelike, Utc};
use monty::{
    ExcType, MontyDate, MontyDateTime, MontyFileHandle, MontyObject, OsFunctionCall, dir_stat,
    file_stat, symlink_stat,
};
use std::path::Path;

/// Markdown description of the supported `pathlib.Path` and `os` operations.
///
/// Signatures reflect what Monty actually forwards to the host: it performs
/// Python-level argument binding via `FromArgs` before dispatch and rejects
/// arguments the host doesn't model (e.g. `follow_symlinks`, `encoding`,
/// `errors`, `newline`, `strict`, `missing_ok`).
pub(crate) const OS_CALLS_DESCRIPTION: &str = "\
Supported `pathlib.Path` operations:\n\
- `Path.exists() -> bool`\n\
- `Path.is_file() -> bool`\n\
- `Path.is_dir() -> bool`\n\
- `Path.is_symlink() -> bool`\n\
- `Path.read_text() -> str` (UTF-8)\n\
- `Path.read_bytes() -> bytes`\n\
- `Path.write_text(data: str) -> int` (returns number of characters written)\n\
- `Path.write_bytes(data: bytes) -> int` (returns number of bytes written)\n\
- `Path.mkdir(parents: bool = False, exist_ok: bool = False) -> None`\n\
- `Path.unlink() -> None`\n\
- `Path.rmdir() -> None`\n\
- `Path.iterdir() -> list[str]`\n\
- `Path.stat() -> os.stat_result`\n\
- `Path.rename(target: str | Path) -> None`\n\
- `Path.resolve() -> str`\n\
- `Path.absolute() -> str`\n\
\n\
Supported `os` operations:\n\
- `os.getenv(key: str, default=None) -> str | None`\n\
\n\
Supported `datetime` operations:\n\
- `datetime.datetime.now(tz=None) -> datetime.datetime`\n\
- `datetime.date.today() -> datetime.date`\n\
\n\
Supported built-in file I/O:\n\
- `open(file, mode='r') -> file object` (modes: `r`, `rb`, `w`, `wb`, `a`, `ab`)\n\
- `file.read()`, `file.write(...)`, `file.close()` and related methods\n\
- text `write()` returns the number of characters written; binary `write()` returns the number of bytes";

/// Small helper to wrap an `io::Error` in an `OSError`-typed `MontyObject`.
fn os_err(e: std::io::Error) -> MontyObject {
    MontyObject::Exception {
        exc_type: ExcType::OSError,
        arg: Some(format!("{e}")),
    }
}

/// Look up an Extism plugin-config value, isolated behind a tiny shim so the
/// native test build (which can't link Extism host imports) can swap in a stub.
#[cfg(not(test))]
fn extism_config_get(key: &str) -> Result<Option<String>, String> {
    extism_pdk::config::get(key).map_err(|e| e.to_string())
}

#[cfg(test)]
fn extism_config_get(_key: &str) -> Result<Option<String>, String> {
    // Native test builds can't link Extism host imports; emulate "no config"
    // so the `Getenv` arm falls back to the caller's `default`.
    Ok(None)
}

/// Handle OsCalls from the Monty VM.
///
/// Monty hands us a tagged [`OsFunctionCall`] whose variants carry the typed
/// args directly, and performs all Python-level argument binding (positional/
/// keyword resolution, defaults, arity, type checks) via `FromArgs` *before*
/// dispatch. So we receive only the resolved typed fields and just service the
/// call.
pub(crate) fn handle_os_call(call: OsFunctionCall) -> MontyObject {
    match &call {
        // ---- Path property checks --------------------------------------
        OsFunctionCall::Exists(p) => MontyObject::Bool(Path::new(p.as_str()).exists()),
        OsFunctionCall::IsFile(p) => MontyObject::Bool(Path::new(p.as_str()).is_file()),
        OsFunctionCall::IsDir(p) => MontyObject::Bool(Path::new(p.as_str()).is_dir()),
        OsFunctionCall::IsSymlink(p) => MontyObject::Bool(Path::new(p.as_str()).is_symlink()),

        // ---- Read --------------------------------------------------------
        OsFunctionCall::ReadText(p) => match std::fs::read_to_string(p.as_str()) {
            Ok(contents) => MontyObject::String(contents),
            Err(e) => os_err(e),
        },
        OsFunctionCall::ReadBytes(p) => match std::fs::read(p.as_str()) {
            Ok(bytes) => MontyObject::Bytes(bytes),
            Err(e) => os_err(e),
        },

        // ---- Write / Append ---------------------------------------------
        // Text-mode `file.write` / `Path.write_text` return the number of
        // *characters* written; Monty's `apply_write_position` uses this to
        // advance the file-handle position. Binary returns byte count.
        OsFunctionCall::WriteText(args) => {
            let char_count = args.data.chars().count() as i64;
            match std::fs::write(args.path.as_str(), args.data.as_bytes()) {
                Ok(()) => MontyObject::Int(char_count),
                Err(e) => os_err(e),
            }
        }
        OsFunctionCall::WriteBytes(args) => {
            let byte_count = args.data.len() as i64;
            match std::fs::write(args.path.as_str(), args.data.as_slice()) {
                Ok(()) => MontyObject::Int(byte_count),
                Err(e) => os_err(e),
            }
        }
        OsFunctionCall::AppendText(args) => {
            let char_count = args.data.chars().count() as i64;
            match std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(args.path.as_str())
                .and_then(|mut f| std::io::Write::write_all(&mut f, args.data.as_bytes()))
            {
                Ok(()) => MontyObject::Int(char_count),
                Err(e) => os_err(e),
            }
        }
        OsFunctionCall::AppendBytes(args) => {
            let byte_count = args.data.len() as i64;
            match std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(args.path.as_str())
                .and_then(|mut f| std::io::Write::write_all(&mut f, args.data.as_slice()))
            {
                Ok(()) => MontyObject::Int(byte_count),
                Err(e) => os_err(e),
            }
        }

        // ---- open() ------------------------------------------------------
        // The host never holds a live OS handle — perform the open-time
        // effect (truncate `w`, create `a`, existence check `r`) and return a
        // stateless [`MontyFileHandle`]. Subsequent reads/writes arrive as
        // their own OS calls keyed by this path.
        OsFunctionCall::Open(args) => {
            let path = args.path.as_str();
            let mode = args.mode;
            let p = Path::new(path);
            if !mode.truncate() && !mode.create() && p.is_dir() {
                MontyObject::Exception {
                    exc_type: ExcType::IsADirectoryError,
                    arg: Some(format!("[Errno 21] Is a directory: '{path}'")),
                }
            } else {
                let effect = if mode.truncate() {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(p)
                        .map(drop)
                } else if mode.create() {
                    std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(p)
                        .map(drop)
                } else {
                    std::fs::OpenOptions::new().read(true).open(p).map(drop)
                };
                match effect {
                    Ok(()) => MontyObject::FileHandle(MontyFileHandle {
                        path: path.to_owned(),
                        mode,
                        position: 0,
                    }),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => MontyObject::Exception {
                        exc_type: ExcType::FileNotFoundError,
                        arg: Some(format!("[Errno 2] No such file or directory: '{path}'")),
                    },
                    Err(e) => os_err(e),
                }
            }
        }

        // ---- Directory & filesystem mutate ------------------------------
        OsFunctionCall::Mkdir(args) => {
            let result = if args.parents {
                std::fs::create_dir_all(args.path.as_str())
            } else {
                std::fs::create_dir(args.path.as_str())
            };
            match result {
                Ok(()) => MontyObject::None,
                Err(e) if args.exist_ok && e.kind() == std::io::ErrorKind::AlreadyExists => {
                    MontyObject::None
                }
                Err(e) => os_err(e),
            }
        }
        OsFunctionCall::Unlink(p) => match std::fs::remove_file(p.as_str()) {
            Ok(()) => MontyObject::None,
            Err(e) => os_err(e),
        },
        OsFunctionCall::Rmdir(p) => match std::fs::remove_dir(p.as_str()) {
            Ok(()) => MontyObject::None,
            Err(e) => os_err(e),
        },
        OsFunctionCall::Iterdir(p) => match std::fs::read_dir(p.as_str()) {
            Ok(entries) => MontyObject::List(
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| MontyObject::String(e.path().to_string_lossy().into_owned()))
                    .collect(),
            ),
            Err(e) => os_err(e),
        },
        OsFunctionCall::Rename(args) => {
            match std::fs::rename(args.src.as_str(), args.dst.as_str()) {
                Ok(()) => MontyObject::None,
                Err(e) => os_err(e),
            }
        }

        // ---- Stat -------------------------------------------------------
        // Monty no longer forwards `follow_symlinks`; always follow.
        OsFunctionCall::Stat(p) => match std::fs::metadata(p.as_str()) {
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
            Err(e) => os_err(e),
        },

        // ---- Resolve / Absolute -----------------------------------------
        // Monty no longer forwards `strict`; resolve what we can without
        // requiring existence (matches `Path.resolve(strict=False)`).
        OsFunctionCall::Resolve(p) => {
            let path = Path::new(p.as_str());
            MontyObject::String(
                std::env::current_dir()
                    .map(|cwd| {
                        let abs = if path.is_absolute() {
                            path.to_path_buf()
                        } else {
                            cwd.join(path)
                        };
                        std::fs::canonicalize(&abs)
                            .unwrap_or(abs)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|_| p.as_str().to_string()),
            )
        }
        OsFunctionCall::Absolute(p) => {
            let path = Path::new(p.as_str());
            MontyObject::String(
                std::env::current_dir()
                    .map(|cwd| cwd.join(path).to_string_lossy().into_owned())
                    .unwrap_or_else(|_| p.as_str().to_string()),
            )
        }

        // ---- Environment ------------------------------------------------
        OsFunctionCall::Getenv(args) => match extism_config_get(&args.key) {
            Ok(Some(val)) => MontyObject::String(val),
            Ok(None) => args.default.clone(),
            Err(e) => MontyObject::Exception {
                exc_type: ExcType::OSError,
                arg: Some(format!("os.getenv: Error getting {}: {e}", args.key)),
            },
        },
        OsFunctionCall::GetEnviron => MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some("OS function os.environ is not implemented in this runtime".into()),
        },

        // ---- Date / Time ------------------------------------------------
        OsFunctionCall::DateTimeNow(tz) => match tz {
            MontyObject::TimeZone(tz) => match FixedOffset::east_opt(tz.offset_seconds) {
                Some(offset) => {
                    let now = Utc::now().with_timezone(&offset);
                    MontyObject::DateTime(MontyDateTime {
                        year: now.year(),
                        month: now.month() as u8,
                        day: now.day() as u8,
                        hour: now.hour() as u8,
                        minute: now.minute() as u8,
                        second: now.second() as u8,
                        microsecond: now.timestamp_subsec_micros(),
                        offset_seconds: Some(now.offset().local_minus_utc()),
                        timezone_name: tz.name.clone(),
                    })
                }
                None => MontyObject::Exception {
                    exc_type: ExcType::ValueError,
                    arg: Some(format!(
                        "'tz' contains an invalid offset ({})",
                        tz.offset_seconds
                    )),
                },
            },
            MontyObject::None => {
                let now = Local::now();
                MontyObject::DateTime(MontyDateTime {
                    year: now.year(),
                    month: now.month() as u8,
                    day: now.day() as u8,
                    hour: now.hour() as u8,
                    minute: now.minute() as u8,
                    second: now.second() as u8,
                    microsecond: now.timestamp_subsec_micros(),
                    offset_seconds: Some(now.offset().local_minus_utc()),
                    timezone_name: None,
                })
            }
            _ => MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some("datetime.now: 'tz' must be a timezone".into()),
            },
        },
        OsFunctionCall::DateToday => {
            let today = Local::now().date_naive();
            MontyObject::Date(MontyDate {
                year: today.year(),
                month: today.month() as u8,
                day: today.day() as u8,
            })
        }

        // ---- Anything else (currently just `Used`, unreachable) ---------
        other => MontyObject::Exception {
            exc_type: ExcType::OSError,
            arg: Some(format!(
                "OS function {other} is not implemented in this runtime"
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monty::{
        GetenvArgs, MkdirCallArgs, MontyFileHandle, MontyPath, MontyTimeZone, OpenCallArgs,
        PathBytesDataArgs, PathStringDataArgs, RenameCallArgs,
    };
    use std::io::Write;
    use tempfile::TempDir;

    // ── Helpers ──────────────────────────────────────────────────────

    /// Build a `MontyPath` from a borrowed string slice.
    fn mp(s: &str) -> MontyPath {
        MontyPath::new(s.to_string())
    }

    /// Shorthand for invoking the real dispatcher.
    fn call(c: OsFunctionCall) -> MontyObject {
        handle_os_call(c)
    }

    fn is_bool(obj: &MontyObject, expected: bool) -> bool {
        matches!(obj, MontyObject::Bool(b) if *b == expected)
    }

    fn is_exception_of(obj: &MontyObject, expected: ExcType) -> bool {
        matches!(obj, MontyObject::Exception { exc_type, .. } if *exc_type == expected)
    }

    /// Create a temp file inside `dir` with the given name and content;
    /// returns its absolute path as a `String`.
    fn create_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path.to_string_lossy().into_owned()
    }

    /// Return the string path for a child inside a `TempDir` (doesn't create it).
    fn child_path(dir: &TempDir, name: &str) -> String {
        dir.path().join(name).to_string_lossy().into_owned()
    }

    // ── Exists ──────────────────────────────────────────────────────

    #[test]
    fn exists_true_for_existing_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "exists.txt", "hi");
        assert!(is_bool(&call(OsFunctionCall::Exists(mp(&p))), true));
    }

    #[test]
    fn exists_true_for_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        assert!(is_bool(&call(OsFunctionCall::Exists(mp(&p))), true));
    }

    #[test]
    fn exists_false_for_nonexistent() {
        assert!(is_bool(
            &call(OsFunctionCall::Exists(mp("/tmp/__no_such_file__"))),
            false,
        ));
    }

    // ── IsFile / IsDir / IsSymlink ──────────────────────────────────

    #[test]
    fn is_file_true_for_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "data");
        assert!(is_bool(&call(OsFunctionCall::IsFile(mp(&p))), true));
    }

    #[test]
    fn is_file_false_for_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        assert!(is_bool(&call(OsFunctionCall::IsFile(mp(&p))), false));
    }

    #[test]
    fn is_file_false_for_nonexistent() {
        assert!(is_bool(
            &call(OsFunctionCall::IsFile(mp("/tmp/__nope__"))),
            false,
        ));
    }

    #[test]
    fn is_dir_true_for_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        assert!(is_bool(&call(OsFunctionCall::IsDir(mp(&p))), true));
    }

    #[test]
    fn is_dir_false_for_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "x");
        assert!(is_bool(&call(OsFunctionCall::IsDir(mp(&p))), false));
    }

    #[test]
    fn is_dir_false_for_nonexistent() {
        assert!(is_bool(
            &call(OsFunctionCall::IsDir(mp("/tmp/__nope_dir__"))),
            false,
        ));
    }

    #[test]
    fn is_symlink_false_for_regular_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "plain.txt", "x");
        assert!(is_bool(&call(OsFunctionCall::IsSymlink(mp(&p))), false));
    }

    #[test]
    #[cfg(unix)]
    fn is_symlink_true_for_symlink() {
        let dir = TempDir::new().unwrap();
        let target = create_file(&dir, "target.txt", "x");
        let link = child_path(&dir, "link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(is_bool(&call(OsFunctionCall::IsSymlink(mp(&link))), true));
    }

    // ── ReadText / ReadBytes ────────────────────────────────────────

    #[test]
    fn read_text_success() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hello");
        let r = call(OsFunctionCall::ReadText(mp(&p)));
        assert!(matches!(r, MontyObject::String(ref s) if s == "hello"));
    }

    #[test]
    fn read_text_nonexistent_returns_oserror() {
        let r = call(OsFunctionCall::ReadText(mp("/tmp/__no_such_read__")));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    #[test]
    fn read_bytes_success() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "data.bin", "binary");
        let r = call(OsFunctionCall::ReadBytes(mp(&p)));
        assert!(matches!(r, MontyObject::Bytes(ref b) if b == b"binary"));
    }

    #[test]
    fn read_bytes_nonexistent_returns_oserror() {
        let r = call(OsFunctionCall::ReadBytes(mp("/tmp/__no_such_bytes__")));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── WriteText / WriteBytes ──────────────────────────────────────

    #[test]
    fn write_text_returns_char_count_and_writes_file() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.txt");
        let r = call(OsFunctionCall::WriteText(PathStringDataArgs {
            path: mp(&p),
            data: "written content".into(),
        }));
        // 15 characters
        assert!(matches!(r, MontyObject::Int(15)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "written content");
    }

    #[test]
    fn write_text_counts_characters_not_bytes() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "uni.txt");
        let r = call(OsFunctionCall::WriteText(PathStringDataArgs {
            path: mp(&p),
            // "🎉" is one character but four UTF-8 bytes.
            data: "🎉".into(),
        }));
        assert!(matches!(r, MontyObject::Int(1)));
    }

    #[test]
    fn write_text_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "old content");
        let r = call(OsFunctionCall::WriteText(PathStringDataArgs {
            path: mp(&p),
            data: "new".into(),
        }));
        assert!(matches!(r, MontyObject::Int(3)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
    }

    #[test]
    fn write_bytes_returns_byte_count_and_writes_file() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "out.bin");
        let r = call(OsFunctionCall::WriteBytes(PathBytesDataArgs {
            path: mp(&p),
            data: vec![1, 2, 3],
        }));
        assert!(matches!(r, MontyObject::Int(3)));
        assert_eq!(std::fs::read(&p).unwrap(), vec![1, 2, 3]);
    }

    // ── AppendText / AppendBytes ────────────────────────────────────

    #[test]
    fn append_text_appends_and_returns_char_count() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "log.txt", "a");
        let r = call(OsFunctionCall::AppendText(PathStringDataArgs {
            path: mp(&p),
            data: "bc".into(),
        }));
        assert!(matches!(r, MontyObject::Int(2)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "abc");
    }

    #[test]
    fn append_text_creates_missing_file() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "new.txt");
        let r = call(OsFunctionCall::AppendText(PathStringDataArgs {
            path: mp(&p),
            data: "hi".into(),
        }));
        assert!(matches!(r, MontyObject::Int(2)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "hi");
    }

    #[test]
    fn append_text_counts_characters_not_bytes() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "uni.txt");
        let r = call(OsFunctionCall::AppendText(PathStringDataArgs {
            path: mp(&p),
            data: "🎉".into(),
        }));
        assert!(matches!(r, MontyObject::Int(1)));
    }

    #[test]
    fn append_bytes_appends_and_returns_byte_count() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "log.bin", "");
        let r = call(OsFunctionCall::AppendBytes(PathBytesDataArgs {
            path: mp(&p),
            data: vec![1, 2, 3],
        }));
        assert!(matches!(r, MontyObject::Int(3)));
        assert_eq!(std::fs::read(&p).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn append_bytes_appends_to_existing() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "acc.bin");
        let _ = call(OsFunctionCall::AppendBytes(PathBytesDataArgs {
            path: mp(&p),
            data: vec![1],
        }));
        let r = call(OsFunctionCall::AppendBytes(PathBytesDataArgs {
            path: mp(&p),
            data: vec![2, 3],
        }));
        assert!(matches!(r, MontyObject::Int(2)));
        assert_eq!(std::fs::read(&p).unwrap(), vec![1, 2, 3]);
    }

    // ── open() ──────────────────────────────────────────────────────

    fn open_args(path: &str, mode: &str) -> OpenCallArgs {
        OpenCallArgs {
            path: mp(path),
            mode: mode.parse().unwrap(),
        }
    }

    #[test]
    fn open_read_existing_returns_handle() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "r.txt", "hi");
        let r = call(OsFunctionCall::Open(open_args(&p, "r")));
        assert!(matches!(r, MontyObject::FileHandle(_)));
    }

    #[test]
    fn open_read_nonexistent_raises_file_not_found() {
        let r = call(OsFunctionCall::Open(open_args(
            "/tmp/__no_such_open_target__",
            "r",
        )));
        assert!(is_exception_of(&r, ExcType::FileNotFoundError));
    }

    #[test]
    fn open_read_directory_raises_is_a_directory() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        let r = call(OsFunctionCall::Open(open_args(&p, "r")));
        assert!(is_exception_of(&r, ExcType::IsADirectoryError));
    }

    #[test]
    fn open_write_truncates_existing_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "w.txt", "existing");
        let r = call(OsFunctionCall::Open(open_args(&p, "w")));
        assert!(matches!(r, MontyObject::FileHandle(_)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "");
    }

    #[test]
    fn open_write_creates_missing_file() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "new.txt");
        let r = call(OsFunctionCall::Open(open_args(&p, "w")));
        assert!(matches!(r, MontyObject::FileHandle(_)));
        assert!(Path::new(&p).is_file());
    }

    #[test]
    fn open_append_preserves_existing_content() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "a.txt", "keep");
        let r = call(OsFunctionCall::Open(open_args(&p, "a")));
        assert!(matches!(r, MontyObject::FileHandle(_)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "keep");
    }

    #[test]
    fn open_handle_carries_path_and_zero_position() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "h.txt", "x");
        if let MontyObject::FileHandle(MontyFileHandle { path, position, .. }) =
            call(OsFunctionCall::Open(open_args(&p, "r")))
        {
            assert_eq!(path, p);
            assert_eq!(position, 0);
        } else {
            panic!("expected FileHandle");
        }
    }

    // ── Mkdir ───────────────────────────────────────────────────────

    fn mkdir_args(path: &str, parents: bool, exist_ok: bool) -> MkdirCallArgs {
        MkdirCallArgs {
            path: mp(path),
            parents,
            exist_ok,
        }
    }

    #[test]
    fn mkdir_creates_directory() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "newdir");
        let r = call(OsFunctionCall::Mkdir(mkdir_args(&p, false, false)));
        assert!(matches!(r, MontyObject::None));
        assert!(Path::new(&p).is_dir());
    }

    #[test]
    fn mkdir_fails_without_parents_for_nested() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "a/b/c");
        let r = call(OsFunctionCall::Mkdir(mkdir_args(&p, false, false)));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    #[test]
    fn mkdir_with_parents_creates_nested() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "x/y/z");
        let r = call(OsFunctionCall::Mkdir(mkdir_args(&p, true, false)));
        assert!(matches!(r, MontyObject::None));
        assert!(Path::new(&p).is_dir());
    }

    #[test]
    fn mkdir_already_exists_no_exist_ok_fails() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "ex");
        std::fs::create_dir(&p).unwrap();
        let r = call(OsFunctionCall::Mkdir(mkdir_args(&p, false, false)));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    #[test]
    fn mkdir_already_exists_with_exist_ok_succeeds() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "ex");
        std::fs::create_dir(&p).unwrap();
        let r = call(OsFunctionCall::Mkdir(mkdir_args(&p, false, true)));
        assert!(matches!(r, MontyObject::None));
    }

    // ── Unlink ──────────────────────────────────────────────────────

    #[test]
    fn unlink_removes_file() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "rm.txt", "");
        let r = call(OsFunctionCall::Unlink(mp(&p)));
        assert!(matches!(r, MontyObject::None));
        assert!(!Path::new(&p).exists());
    }

    #[test]
    fn unlink_fails_on_nonexistent() {
        // Monty no longer forwards `missing_ok`, so this always errors.
        let r = call(OsFunctionCall::Unlink(mp("/tmp/__no_such_unlink__")));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── Rmdir ───────────────────────────────────────────────────────

    #[test]
    fn rmdir_removes_empty_directory() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "empty");
        std::fs::create_dir(&p).unwrap();
        let r = call(OsFunctionCall::Rmdir(mp(&p)));
        assert!(matches!(r, MontyObject::None));
        assert!(!Path::new(&p).exists());
    }

    #[test]
    fn rmdir_fails_on_nonempty_directory() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "nonempty");
        std::fs::create_dir(&p).unwrap();
        let _ = create_file(&dir, "nonempty/file.txt", "x");
        let r = call(OsFunctionCall::Rmdir(mp(&p)));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    #[test]
    fn rmdir_fails_on_nonexistent() {
        let r = call(OsFunctionCall::Rmdir(mp("/tmp/__no_such_rmdir__")));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── Iterdir ─────────────────────────────────────────────────────

    #[test]
    fn iterdir_lists_contents() {
        let dir = TempDir::new().unwrap();
        let _ = create_file(&dir, "a.txt", "");
        let _ = create_file(&dir, "b.txt", "");
        let p = dir.path().to_string_lossy().into_owned();
        let r = call(OsFunctionCall::Iterdir(mp(&p)));
        match r {
            MontyObject::List(items) => {
                assert_eq!(items.len(), 2);
                let mut names: Vec<String> = items
                    .into_iter()
                    .map(|o| match o {
                        MontyObject::String(s) => std::path::Path::new(&s)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or(s),
                        other => panic!("expected String entry, got {other:?}"),
                    })
                    .collect();
                names.sort();
                assert_eq!(names, vec!["a.txt", "b.txt"]);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn iterdir_empty_directory_returns_empty_list() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        let r = call(OsFunctionCall::Iterdir(mp(&p)));
        assert!(matches!(r, MontyObject::List(ref items) if items.is_empty()));
    }

    #[test]
    fn iterdir_nonexistent_returns_oserror() {
        // Previously returned an empty list when the path was `None`. Monty
        // now always provides a typed path; a missing directory surfaces as
        // a real OS error.
        let r = call(OsFunctionCall::Iterdir(mp("/tmp/__no_such_iter__")));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── Stat ────────────────────────────────────────────────────────

    #[test]
    fn stat_file_returns_stat_result() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "f.txt", "hi");
        let r = call(OsFunctionCall::Stat(mp(&p)));
        // `file_stat` returns a NamedTuple wrapper; we just verify the call
        // produced a non-error, non-None value.
        assert!(!matches!(r, MontyObject::Exception { .. }));
        assert!(!matches!(r, MontyObject::None));
    }

    #[test]
    fn stat_directory_returns_stat_result() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        let r = call(OsFunctionCall::Stat(mp(&p)));
        assert!(!matches!(r, MontyObject::Exception { .. }));
    }

    #[test]
    fn stat_nonexistent_returns_oserror() {
        let r = call(OsFunctionCall::Stat(mp("/tmp/__no_such_stat__")));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── Rename ──────────────────────────────────────────────────────

    #[test]
    fn rename_success() {
        let dir = TempDir::new().unwrap();
        let src = create_file(&dir, "from.txt", "data");
        let dst = child_path(&dir, "to.txt");
        let r = call(OsFunctionCall::Rename(RenameCallArgs {
            src: mp(&src),
            dst: mp(&dst),
        }));
        assert!(matches!(r, MontyObject::None));
        assert!(!Path::new(&src).exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "data");
    }

    #[test]
    fn rename_nonexistent_source_returns_oserror() {
        let dir = TempDir::new().unwrap();
        let dst = child_path(&dir, "to.txt");
        let r = call(OsFunctionCall::Rename(RenameCallArgs {
            src: mp("/tmp/__no_such_rename_src__"),
            dst: mp(&dst),
        }));
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── Resolve / Absolute ──────────────────────────────────────────

    #[test]
    fn resolve_existing_file_returns_canonical_string() {
        let dir = TempDir::new().unwrap();
        let p = create_file(&dir, "r.txt", "x");
        let r = call(OsFunctionCall::Resolve(mp(&p)));
        match r {
            MontyObject::String(s) => {
                assert!(s.contains("r.txt"));
                assert!(Path::new(&s).is_absolute());
            }
            other => panic!("expected String, got {other:?}"),
        }
    }

    #[test]
    fn resolve_nonexistent_returns_a_path_string() {
        // Non-strict: never fails even if the target doesn't exist.
        let r = call(OsFunctionCall::Resolve(mp("/tmp/__no_such_resolve__")));
        assert!(matches!(r, MontyObject::String(_)));
    }

    #[test]
    fn absolute_returns_absolute_path_for_relative_input() {
        let r = call(OsFunctionCall::Absolute(mp("relative.txt")));
        match r {
            MontyObject::String(s) => assert!(Path::new(&s).is_absolute()),
            other => panic!("expected String, got {other:?}"),
        }
    }

    #[test]
    fn absolute_keeps_absolute_input() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().to_string_lossy().into_owned();
        let r = call(OsFunctionCall::Absolute(mp(&p)));
        match r {
            MontyObject::String(s) => assert!(s.contains(&p)),
            other => panic!("expected String, got {other:?}"),
        }
    }

    // ── Roundtrips ──────────────────────────────────────────────────

    #[test]
    fn write_text_then_read_text_roundtrip() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "roundtrip.txt");
        let content = "hello\nworld\n🎉";
        let w = call(OsFunctionCall::WriteText(PathStringDataArgs {
            path: mp(&p),
            data: content.into(),
        }));
        // 13 characters: "hello" + "\n" + "world" + "\n" + "🎉"
        assert!(matches!(w, MontyObject::Int(13)));
        let r = call(OsFunctionCall::ReadText(mp(&p)));
        assert!(matches!(r, MontyObject::String(ref s) if s == content));
    }

    #[test]
    fn write_bytes_then_read_bytes_roundtrip() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "roundtrip.bin");
        let data = vec![0, 1, 127, 128, 255];
        let w = call(OsFunctionCall::WriteBytes(PathBytesDataArgs {
            path: mp(&p),
            data: data.clone(),
        }));
        assert!(matches!(w, MontyObject::Int(5)));
        let r = call(OsFunctionCall::ReadBytes(mp(&p)));
        assert!(matches!(r, MontyObject::Bytes(ref b) if *b == data));
    }

    #[test]
    fn mkdir_iterdir_rmdir_lifecycle() {
        let dir = TempDir::new().unwrap();
        let p = child_path(&dir, "lifecycle");
        assert!(matches!(
            call(OsFunctionCall::Mkdir(mkdir_args(&p, false, false))),
            MontyObject::None
        ));
        assert!(matches!(
            call(OsFunctionCall::Iterdir(mp(&p))),
            MontyObject::List(ref items) if items.is_empty()
        ));
        assert!(matches!(
            call(OsFunctionCall::Rmdir(mp(&p))),
            MontyObject::None
        ));
        assert!(!Path::new(&p).exists());
    }

    // ── Environment ─────────────────────────────────────────────────

    #[test]
    fn getenv_unset_returns_string_default() {
        // The test-build stub for `extism_config_get` always returns
        // `Ok(None)`, so the caller's `default` is what comes back.
        let r = call(OsFunctionCall::Getenv(GetenvArgs {
            key: "NEVER_SET".into(),
            default: MontyObject::String("fallback".into()),
        }));
        assert!(matches!(r, MontyObject::String(ref s) if s == "fallback"));
    }

    #[test]
    fn getenv_unset_with_none_default() {
        let r = call(OsFunctionCall::Getenv(GetenvArgs {
            key: "NEVER_SET".into(),
            default: MontyObject::None,
        }));
        assert!(matches!(r, MontyObject::None));
    }

    #[test]
    fn get_environ_returns_unsupported_oserror() {
        let r = call(OsFunctionCall::GetEnviron);
        assert!(is_exception_of(&r, ExcType::OSError));
    }

    // ── Date / Time ─────────────────────────────────────────────────

    #[test]
    fn date_today_returns_date_variant_in_valid_ranges() {
        let r = call(OsFunctionCall::DateToday);
        match r {
            MontyObject::Date(d) => {
                assert!(d.year >= 2024);
                assert!(d.month >= 1 && d.month <= 12);
                assert!(d.day >= 1 && d.day <= 31);
            }
            other => panic!("expected Date, got {other:?}"),
        }
    }

    #[test]
    fn datetime_now_no_tz_returns_local_datetime() {
        let r = call(OsFunctionCall::DateTimeNow(MontyObject::None));
        match r {
            MontyObject::DateTime(dt) => {
                assert!(dt.year >= 2024);
                assert!(dt.timezone_name.is_none());
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn datetime_now_with_utc_timezone() {
        let r = call(OsFunctionCall::DateTimeNow(MontyObject::TimeZone(
            MontyTimeZone {
                offset_seconds: 0,
                name: Some("UTC".into()),
            },
        )));
        match r {
            MontyObject::DateTime(dt) => {
                assert_eq!(dt.offset_seconds, Some(0));
                assert_eq!(dt.timezone_name, Some("UTC".into()));
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn datetime_now_with_positive_offset_timezone() {
        let r = call(OsFunctionCall::DateTimeNow(MontyObject::TimeZone(
            MontyTimeZone {
                offset_seconds: 3 * 3600,
                name: Some("EAT".into()),
            },
        )));
        match r {
            MontyObject::DateTime(dt) => assert_eq!(dt.offset_seconds, Some(3 * 3600)),
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn datetime_now_with_negative_offset_timezone() {
        let r = call(OsFunctionCall::DateTimeNow(MontyObject::TimeZone(
            MontyTimeZone {
                offset_seconds: -5 * 3600,
                name: Some("EST".into()),
            },
        )));
        match r {
            MontyObject::DateTime(dt) => assert_eq!(dt.offset_seconds, Some(-5 * 3600)),
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn datetime_now_invalid_offset_returns_value_error() {
        let r = call(OsFunctionCall::DateTimeNow(MontyObject::TimeZone(
            MontyTimeZone {
                offset_seconds: 1_000_000, // far outside ±24h
                name: None,
            },
        )));
        assert!(is_exception_of(&r, ExcType::ValueError));
    }

    #[test]
    fn datetime_now_wrong_tz_type_returns_type_error() {
        let r = call(OsFunctionCall::DateTimeNow(MontyObject::Int(0)));
        assert!(is_exception_of(&r, ExcType::TypeError));
    }
}

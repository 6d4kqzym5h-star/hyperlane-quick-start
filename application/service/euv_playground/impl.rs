use super::*;

/// All the euv-playground service methods live on
/// [`EuvPlaygroundService`] — a stateless zero-sized type so callers
/// can pass it around as `Copy`.
impl EuvPlaygroundService {
    /// URL-encodes a numeric id so the on-disk directory and the API
    /// response are both safe to use as URL path components. Mirrors the
    /// `Encode::execute(CHARSETS, &id.to_string())` convention used by the
    /// `auth` and `rss` services.
    ///
    /// # Arguments
    ///
    /// - `i64` - The numeric id to encode.
    ///
    /// # Returns
    ///
    /// - `String`: The encoded string. Falls back to the plain
    ///   `id.to_string()` if the underlying `Encode::execute` call
    ///   fails so the playground never blocks on an encoding error.
    #[instrument_trace]
    pub fn encode_id(id: i64) -> String {
        Encode::execute(CHARSETS, &id.to_string())
            .map_err(|_: EncodeError| ERROR_FAILED_TO_ENCODE_ID.to_string())
            .map(|encoded: String| {
                if encoded.is_empty() {
                    id.to_string()
                } else {
                    encoded
                }
            })
            .unwrap_or_else(|_| id.to_string())
    }

    /// Inverse of [`Self::encode_id`]. Decodes a previously URL-encoded
    /// id back into its original `i64`. Falls back to a plain `i64` parse
    /// for backward compatibility with the (legacy) un-encoded form.
    ///
    /// # Arguments
    ///
    /// - `&str` - The encoded id.
    ///
    /// # Returns
    ///
    /// - `Result<i64, String>`: The decoded numeric id, or an error if
    ///   the format is invalid.
    #[instrument_trace]
    pub fn decode_id(encoded: &str) -> Result<i64, String> {
        let decoded: String = Decode::execute(CHARSETS, encoded)
            .map_err(|_: DecodeError| ERROR_INVALID_ID_FORMAT.to_string())
            .unwrap_or_else(|_| encoded.to_string());
        decoded
            .parse::<i64>()
            .map_err(|_: ParseIntError| ERROR_INVALID_ID_FORMAT.to_string())
    }

    /// URL-encodes a build job id the same way [`Self::encode_id`] does
    /// for project ids, so URL path components and response fields can
    /// travel through the network as plain ASCII.
    ///
    /// # Arguments
    ///
    /// - `BuildJobId` - The numeric build job id.
    ///
    /// # Returns
    ///
    /// - `String`: The encoded job id; never fails (mirrors
    ///   [`Self::encode_id`]).
    #[instrument_trace]
    pub fn encode_job_id(job_id: BuildJobId) -> String {
        Encode::execute(CHARSETS, &job_id.to_string())
            .map(|encoded: String| {
                if encoded.is_empty() {
                    job_id.to_string()
                } else {
                    encoded
                }
            })
            .unwrap_or_else(|_| job_id.to_string())
    }

    /// Inverse of [`Self::encode_job_id`]. Decodes an obfuscated job id
    /// string back into its numeric `u64`. Falls back to a plain `u64`
    /// parse for backward compatibility with callers that pass an un-
    /// encoded id.
    ///
    /// # Arguments
    ///
    /// - `&str` - The encoded job id string.
    ///
    /// # Returns
    ///
    /// - `Result<BuildJobId, String>`: The decoded numeric job id, or an
    ///   error string if the format is invalid.
    #[instrument_trace]
    pub fn decode_job_id(encoded: &str) -> Result<BuildJobId, String> {
        let decoded: String = Decode::execute(CHARSETS, encoded)
            .map(|s: String| s)
            .unwrap_or_else(|_| encoded.to_string());
        decoded
            .parse::<BuildJobId>()
            .map_err(|_: ParseIntError| ERROR_INVALID_ID_FORMAT.to_string())
    }

    /// Current epoch second. Used as a unique component of the build
    /// staging directory name.
    ///
    /// # Returns
    ///
    /// - `u64`: Seconds since the unix epoch, or `0` if the system
    ///   clock is before the epoch (rare, but a safe fallback).
    #[instrument_trace]
    pub fn timestamp_suffix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d: Duration| d.as_secs())
            .unwrap_or(0)
    }

    /// Current epoch millisecond. Used to stamp project metadata.
    ///
    /// # Returns
    ///
    /// - `i64`: Milliseconds since the unix epoch, or `0` on a
    ///   pre-epoch system clock.
    #[instrument_trace]
    pub fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d: Duration| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// Resolves the wasm-pack executable from an explicit override, PATH,
    /// Cargo home, or user home in that order.
    ///
    /// # Arguments
    ///
    /// - `Option<&OsStr>` - Explicit executable override.
    /// - `Option<&OsStr>` - Process PATH value to inspect.
    /// - `Option<&OsStr>` - Cargo installation root.
    /// - `Option<&OsStr>` - User home used for the default `.cargo` root.
    ///
    /// # Returns
    ///
    /// - `PathBuf`: Existing executable path when found, otherwise the bare
    ///   wasm-pack filename so process spawning preserves the normal OS lookup.
    #[instrument_trace]
    pub fn resolve_wasm_pack_binary_from(
        override_path: Option<&OsStr>,
        path: Option<&OsStr>,
        cargo_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> PathBuf {
        if let Some(explicit_path) = override_path.filter(|value: &&OsStr| !value.is_empty()) {
            return PathBuf::from(explicit_path);
        }
        if let Some(path_binary) = path
            .into_iter()
            .flat_map(split_paths)
            .map(|directory: PathBuf| directory.join(EUV_PLAYGROUND_WASM_PACK_BINARY_NAME))
            .find(|candidate: &PathBuf| candidate.is_file())
        {
            return path_binary;
        }
        let cargo_home_binary: Option<PathBuf> =
            cargo_home.map(PathBuf::from).map(|directory: PathBuf| {
                directory
                    .join(EUV_PLAYGROUND_CARGO_BIN_DIR)
                    .join(EUV_PLAYGROUND_WASM_PACK_BINARY_NAME)
            });
        if let Some(candidate) = cargo_home_binary.filter(|candidate: &PathBuf| candidate.is_file())
        {
            return candidate;
        }
        let home_binary: Option<PathBuf> = home.map(PathBuf::from).map(|directory: PathBuf| {
            directory
                .join(EUV_PLAYGROUND_CARGO_HOME_DIR)
                .join(EUV_PLAYGROUND_CARGO_BIN_DIR)
                .join(EUV_PLAYGROUND_WASM_PACK_BINARY_NAME)
        });
        home_binary
            .filter(|candidate: &PathBuf| candidate.is_file())
            .unwrap_or_else(|| PathBuf::from(EUV_PLAYGROUND_WASM_PACK_BINARY_NAME))
    }

    /// Resolves wasm-pack using the current process environment.
    ///
    /// # Returns
    ///
    /// - `PathBuf`: The executable path used for playground builds.
    #[instrument_trace]
    pub fn resolve_wasm_pack_binary() -> PathBuf {
        let override_path: Option<OsString> = var_os(EUV_PLAYGROUND_WASM_PACK_ENV);
        let path: Option<OsString> = var_os(EUV_PLAYGROUND_PATH_ENV);
        let cargo_home: Option<OsString> = var_os(EUV_PLAYGROUND_CARGO_HOME_ENV);
        let home: Option<OsString> =
            var_os(EUV_PLAYGROUND_HOME_ENV).or_else(|| var_os(EUV_PLAYGROUND_USERPROFILE_ENV));
        Self::resolve_wasm_pack_binary_from(
            override_path.as_deref(),
            path.as_deref(),
            cargo_home.as_deref(),
            home.as_deref(),
        )
    }

    /// Drains a spawned `wasm-pack` child's stdout and stderr into two
    /// owned `Vec<u8>`s and joins them with the child's exit-status wait.
    /// All three future is awaited concurrently.
    ///
    /// # Arguments
    ///
    /// - `Child` - The spawned child whose
    ///   stdout/stderr have already been taken into `Option`s.
    ///
    /// # Returns
    ///
    /// - `std::io::Result<Output>`: The captured output
    ///   plus exit status, or the underlying io error.
    #[instrument_trace]
    pub async fn wait_with_output(mut child: Child) -> std::io::Result<Output> {
        use tokio::io::AsyncReadExt;
        let stdout: Option<ChildStdout> = child.stdout.take();
        let stderr: Option<ChildStderr> = child.stderr.take();
        let stdout_task = async move {
            let mut buf: Vec<u8> = Vec::new();
            if let Some(mut s) = stdout {
                let _: Result<usize, Error> = s.read_to_end(&mut buf).await;
            }
            buf
        };
        let stderr_task = async move {
            let mut buf: Vec<u8> = Vec::new();
            if let Some(mut s) = stderr {
                let _: Result<usize, Error> = s.read_to_end(&mut buf).await;
            }
            buf
        };
        let (status, stdout_bytes, stderr_bytes): (
            Result<ExitStatus, std::io::Error>,
            Vec<u8>,
            Vec<u8>,
        ) = tokio::join!(child.wait(), stdout_task, stderr_task,);
        let status: ExitStatus = status?;
        Ok(Output {
            status,
            stdout: stdout_bytes,
            stderr: stderr_bytes,
        })
    }

    /// Writes the submitted code into a fresh temporary crate, runs
    /// `wasm-pack build --target web` in **dev profile** (no
    /// `--release`) so the build is as fast as possible, and publishes
    /// the produced files into
    /// `./resources/static/euv-playground/builds/{project_id}/` so the
    /// existing static-resource route can serve them directly.
    ///
    /// # Arguments
    ///
    /// - `&str` - The user-submitted Rust source.
    /// - `i64` - The owning project id; the build output
    ///   directory is keyed off its encoded form.
    ///
    /// # Returns
    ///
    /// - `Result<PathBuf, String>`: The path to the
    ///   published build directory on success, or a human-readable
    ///   error describing which step failed.
    #[instrument_trace]
    pub async fn build_wasm_pack_output(code: &str, project_id: i64) -> Result<PathBuf, String> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter: u64 = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir_name: String = format!(
            "{}{}-{}{}",
            EUV_PLAYGROUND_BUILD_DIR_PREFIX,
            id(),
            counter,
            Self::timestamp_suffix()
        );
        let dir_path: PathBuf = temp_dir().join(&dir_name);
        let src_dir: PathBuf = dir_path.join("src");
        let www_dir: PathBuf = dir_path.join("www");
        create_dir_all(&src_dir).map_err(|e: std::io::Error| {
            format!("Failed to create src dir {}: {e}", src_dir.display())
        })?;
        create_dir_all(&www_dir).map_err(|e: std::io::Error| {
            format!("Failed to create www dir {}: {e}", www_dir.display())
        })?;
        let cargo_toml_path: PathBuf = dir_path.join("Cargo.toml");
        let lib_rs_path: PathBuf = src_dir.join("lib.rs");
        let index_html_path: PathBuf = www_dir.join("index.html");
        write(&cargo_toml_path, EUV_PLAYGROUND_BUILD_CARGO_TOML)
            .map_err(|e: std::io::Error| format!("Failed to write Cargo.toml: {e}"))?;
        write(&lib_rs_path, code)
            .map_err(|e: std::io::Error| format!("Failed to write src/lib.rs: {e}"))?;
        write(&index_html_path, EUV_PLAYGROUND_BUILD_INDEX_HTML)
            .map_err(|e: std::io::Error| format!("Failed to write www/index.html: {e}"))?;
        let wasm_pack_binary: PathBuf = Self::resolve_wasm_pack_binary();
        let wasm_pack_display: String = wasm_pack_binary.display().to_string();
        let mut cmd: Command = Command::new(&wasm_pack_binary);
        cmd.current_dir(&dir_path)
            .arg("build")
            .arg("--target")
            .arg("web")
            .arg("--dev")
            .arg("--out-dir")
            .arg("www/pkg")
            .env("CARGO_TERM_COLOR", "never")
            .env(
                "CARGO_TARGET_DIR",
                EUV_PLAYGROUND_SHARED_TARGET_DIR.as_os_str(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _: Result<(), Error> = remove_dir_all(&dir_path);
                return Err(format!(
                    "Failed to spawn wasm-pack at {wasm_pack_display}. Install it with `cargo install wasm-pack`, add Cargo's bin directory to PATH, or set {EUV_PLAYGROUND_WASM_PACK_ENV}: {e}"
                ));
            }
        };
        let output = match timeout(
            Duration::from_secs(EUV_PLAYGROUND_BUILD_TIMEOUT_SECS),
            Self::wait_with_output(child),
        )
        .await
        {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                let _: Result<(), Error> = remove_dir_all(&dir_path);
                return Err(format!("wasm-pack wait failed: {e}"));
            }
            Err(_) => {
                let _: Result<(), Error> = remove_dir_all(&dir_path);
                return Err(format!(
                    "wasm-pack timed out after {EUV_PLAYGROUND_BUILD_TIMEOUT_SECS}s"
                ));
            }
        };
        let cleanup = |err: String| -> String {
            let _: Result<(), Error> = remove_dir_all(&dir_path);
            err
        };
        if !output.status.success() {
            let stderr: String = String::from_utf8_lossy(&output.stderr).into_owned();
            let stdout: String = String::from_utf8_lossy(&output.stdout).into_owned();
            let combined: String = if stderr.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            return Err(cleanup(combined));
        }
        let target_dir: PathBuf =
            PathBuf::from(EUV_PLAYGROUND_BUILDS_DIR).join(Self::encode_id(project_id));
        let target_tmp: PathBuf = PathBuf::from(EUV_PLAYGROUND_BUILDS_DIR).join(format!(
            "{}.{}.tmp",
            Self::encode_id(project_id),
            counter
        ));
        if let Err(e) = create_dir_all(PathBuf::from(EUV_PLAYGROUND_BUILDS_DIR)) {
            let _: Result<(), Error> = remove_dir_all(&dir_path);
            return Err(format!(
                "Failed to create builds dir {}: {e}",
                EUV_PLAYGROUND_BUILDS_DIR
            ));
        }
        if let Err(e) = create_dir_all(&target_tmp) {
            let _: Result<(), Error> = remove_dir_all(&dir_path);
            return Err(format!(
                "Failed to create build staging dir {}: {e}",
                target_tmp.display()
            ));
        }
        fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
            create_dir_all(dst)
                .map_err(|e: std::io::Error| format!("mkdir {}: {e}", dst.display()))?;
            for entry in read_dir(src)
                .map_err(|e: std::io::Error| format!("readdir {}: {e}", src.display()))?
            {
                let entry: DirEntry = entry.map_err(|e: std::io::Error| e.to_string())?;
                let from: PathBuf = entry.path();
                let to: PathBuf = dst.join(entry.file_name());
                if from.is_dir() {
                    copy_dir_recursive(&from, &to)?;
                } else {
                    copy(&from, &to).map_err(|e: std::io::Error| {
                        format!("copy {} -> {}: {e}", from.display(), to.display())
                    })?;
                }
            }
            Ok(())
        }
        if let Err(e) = copy_dir_recursive(&www_dir, &target_tmp) {
            let _: Result<(), Error> = remove_dir_all(&dir_path);
            let _: Result<(), Error> = remove_dir_all(&target_tmp);
            return Err(e);
        }
        if target_dir.exists() {
            let _: Result<(), Error> = remove_dir_all(&target_dir);
        }
        if let Err(e) = rename(&target_tmp, &target_dir) {
            let _: Result<(), Error> = remove_dir_all(&dir_path);
            let _: Result<(), Error> = remove_dir_all(&target_tmp);
            return Err(format!(
                "Failed to publish build to {}: {e}",
                target_dir.display()
            ));
        }
        let _: Result<(), Error> = remove_dir_all(&dir_path);
        Ok(target_dir)
    }

    /// Normalizes a user-supplied project name for storage and rendering.
    ///
    /// The name never becomes a file path (projects are stored under
    /// `data/euv_playground/{user_id}/{project_id}/{code.rs,metadata.json}`,
    /// not by name), but it *is* serialized to JSON, echoed back to
    /// the UI, and shown to the user. To keep it safe across all three
    /// uses we:
    ///
    ///   * trim surrounding whitespace,
    ///   * drop ASCII control chars (incl. NUL — JSON/string terminator),
    ///   * drop path separators `/` and `\` so the name can never escape
    ///     `metadata.json` even if a future code path begins to use it
    ///     in a path,
    ///   * drop HTML-active chars `<`, `>`, `&`, `"`, `'` so the frontend
    ///     can render the name with `innerHTML` without escaping,
    ///   * collapse runs of whitespace into a single space,
    ///   * truncate to [`EUV_PLAYGROUND_MAX_NAME_LEN`] chars,
    ///   * fall back to `"Untitled"` if everything was stripped.
    ///
    /// # Arguments
    ///
    /// - `&str` - The user-supplied raw name (may be empty).
    ///
    /// # Returns
    ///
    /// - `String`: The normalized name.
    #[instrument_trace]
    pub fn normalize_name(input: &str) -> String {
        let cleaned: String = input
            .chars()
            .map(|c: char| match c {
                '\t' | '\n' | '\r' => ' ',
                c if (c as u32) < 0x20 => ' ',
                '/' | '\\' | '<' | '>' | '&' | '"' | '\'' => ' ',
                c => c,
            })
            .collect();
        let collapsed: String = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        let trimmed: &str = collapsed.trim();
        let base: &str = if trimmed.is_empty() {
            "Untitled"
        } else {
            trimmed
        };
        if base.chars().count() > EUV_PLAYGROUND_MAX_NAME_LEN {
            base.chars().take(EUV_PLAYGROUND_MAX_NAME_LEN).collect()
        } else {
            base.to_string()
        }
    }

    /// Checks whether a user already owns a project with the normalized name.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The user's playground directory.
    /// - `&str` - The normalized project name to find.
    ///
    /// # Returns
    ///
    /// - `Result<bool, String>`: `true` when a matching project exists,
    ///   otherwise `false`, or a directory-read error.
    #[instrument_trace]
    pub fn project_name_exists(user_dir: &Path, name: &str) -> Result<bool, String> {
        Self::project_name_exists_excluding(user_dir, name, None)
    }

    /// Checks whether another project uses the normalized name.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The user's playground directory.
    /// - `&str` - The normalized project name to find.
    /// - `Option<&Path>` - Project directory to ignore.
    ///
    /// # Returns
    ///
    /// - `Result<bool, String>`: `true` when another match exists,
    ///   otherwise `false`, or a directory-read error.
    #[instrument_trace]
    pub fn project_name_exists_excluding(
        user_dir: &Path,
        name: &str,
        excluded_project_dir: Option<&Path>,
    ) -> Result<bool, String> {
        let entries: ReadDir = read_dir(user_dir).map_err(|error: std::io::Error| {
            format!(
                "Failed to read project directory {}: {error}",
                user_dir.display()
            )
        })?;
        for entry_result in entries {
            let entry: DirEntry = entry_result.map_err(|error: std::io::Error| {
                format!("Failed to read project directory entry: {error}")
            })?;
            let project_dir: PathBuf = entry.path();
            if !project_dir.is_dir()
                || excluded_project_dir
                    .is_some_and(|excluded: &Path| project_dir.as_path() == excluded)
            {
                continue;
            }
            if let Some((existing_name, _updated_at_ms)) = Self::read_metadata(&project_dir)
                && existing_name == name
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Reads the project's `code.rs` from disk.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The project directory.
    ///
    /// # Returns
    ///
    /// - `Result<String, String>`: The file content, or an error.
    #[instrument_trace]
    pub fn read_code(project_dir: &Path) -> Result<String, String> {
        read_to_string(project_dir.join(EUV_PLAYGROUND_CODE_FILE))
            .map_err(|e: std::io::Error| format!("Failed to read code: {e}"))
    }

    /// Writes a project's `code.rs` and updates `metadata.json`'s
    /// timestamp + name.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The project directory.
    /// - `&str` - The normalized project name.
    /// - `&str` - The Rust source code.
    ///
    /// # Returns
    ///
    /// - `Result<i64, String>`: The new `updated_at_ms` timestamp, or
    ///   an error if the file write failed.
    #[instrument_trace]
    pub fn write_project(project_dir: &Path, name: &str, code: &str) -> Result<i64, String> {
        if let Some(parent) = project_dir.parent() {
            let _: Result<(), Error> = create_dir_all(parent);
        }
        if create_dir_all(project_dir).is_err() {
            return Err(format!(
                "Failed to create project dir {}",
                project_dir.display()
            ));
        }
        if write(project_dir.join(EUV_PLAYGROUND_CODE_FILE), code).is_err() {
            return Err(format!(
                "Failed to write code to {}",
                project_dir.join(EUV_PLAYGROUND_CODE_FILE).display()
            ));
        }
        let ts: i64 = Self::now_ms();
        Self::write_metadata(project_dir, name, ts);
        Ok(ts)
    }

    /// Returns the project directory for `(user_id, project_id)`. Does
    /// not create it — callers decide whether to create (write) or
    /// expect-existing (read).
    ///
    /// # Arguments
    ///
    /// - `i32` - The owning user.
    /// - `i64` - The project id (will be encoded).
    ///
    /// # Returns
    ///
    /// - `PathBuf`: The path.
    #[instrument_trace]
    pub fn project_dir(user_id: i32, project_id: i64) -> PathBuf {
        Self::user_dir(user_id).join(Self::encode_id(project_id))
    }

    /// Returns the user's playground directory, creating it if
    /// missing.
    ///
    /// # Arguments
    ///
    /// - `i32` - The owning user (will be encoded).
    ///
    /// # Returns
    ///
    /// - `PathBuf`: The path to the user's playground dir.
    #[instrument_trace]
    pub fn user_dir(user_id: i32) -> PathBuf {
        let p: PathBuf =
            PathBuf::from(EUV_PLAYGROUND_DATA_DIR).join(Self::encode_id(user_id as i64));
        let _: Result<(), Error> = create_dir_all(&p);
        p
    }

    /// Reads `metadata.json` for a project.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The project directory.
    ///
    /// # Returns
    ///
    /// - `Option<(String, i64)>`: `(name, updated_at_ms)`, or `None`
    ///   if the file is missing or unparseable.
    #[instrument_trace]
    pub fn read_metadata(project_dir: &Path) -> Option<(String, i64)> {
        let text: String = read_to_string(project_dir.join(EUV_PLAYGROUND_META_FILE)).ok()?;
        let v: Value = from_str(&text).ok()?;
        let name: String = v
            .get("name")
            .and_then(|x: &Value| x.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let updated_at_ms: i64 = v
            .get("updated_at_ms")
            .and_then(|x: &Value| x.as_i64())
            .unwrap_or(0);
        Some((name, updated_at_ms))
    }

    /// Persists the project metadata to disk as JSON.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The project directory.
    /// - `&str` - The normalized name.
    /// - `i64` - The unix-epoch-ms timestamp.
    #[instrument_trace]
    pub fn write_metadata(project_dir: &Path, name: &str, updated_at_ms: i64) {
        let json: String = format!(
            r#"{{"name":{},"updated_at_ms":{}}}"#,
            to_string(name).unwrap_or_else(|_| "\"Untitled\"".to_string()),
            updated_at_ms
        );
        let _: Result<(), Error> = write(project_dir.join(EUV_PLAYGROUND_META_FILE), json);
    }

    /// Monotonic per-user project-id counter. Persisted to disk in the
    /// user's `_seq` file so deleting the highest-id project doesn't
    /// recycle it.
    ///
    /// # Arguments
    ///
    /// - `&Path` - The user's playground directory.
    ///
    /// # Returns
    ///
    /// - `i64`: The next available project id. Falls back to a
    ///   millisecond-timestamp-based id if the seq file can't be
    ///   written.
    #[instrument_trace]
    pub fn next_project_id(user_dir: &Path) -> i64 {
        let seq_path: PathBuf = user_dir.join(EUV_PLAYGROUND_SEQ_FILE);
        let next: i64 = match read_to_string(&seq_path) {
            Ok(s) => s.trim().parse::<i64>().unwrap_or(0).saturating_add(1),
            Err(_) => 1,
        };
        if let Err(_e) = write(&seq_path, next.to_string()) {
            let ts: i64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d: Duration| d.as_millis() as i64)
                .unwrap_or(0);
            return ts + 1_000_000_000;
        }
        next
    }

    /// Allocates the next monotonic [`BuildJobId`].
    ///
    /// Job ids are server-lifetime unique so the frontend can poll
    /// `/api/euv/playground/run/status/{job_id}` without ambiguity even
    /// after the user has started a second build.
    ///
    /// # Returns
    ///
    /// - `BuildJobId`: The freshly reserved id.
    #[instrument_trace]
    pub fn next_job_id() -> BuildJobId {
        NEXT_BUILD_JOB_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Creates a new `pending` row in [`BUILD_JOB_REGISTRY`] for the
    /// supplied `(user_id, project_id)` pair and returns the freshly
    /// assigned job id.
    ///
    /// # Arguments
    ///
    /// - `i32` - The owning user id.
    /// - `i64` - The owning project id.
    ///
    /// # Returns
    ///
    /// - `BuildJobId`: The new job id; the inserted row starts in
    ///   [`build_status::PENDING`].
    #[instrument_trace]
    pub async fn register_pending_job(user_id: i32, project_id: i64) -> BuildJobId {
        let job_id: BuildJobId = Self::next_job_id();
        let ts: i64 = Self::now_ms();
        let job: BuildJob = BuildJob {
            job_id,
            user_id,
            project_id,
            status: build_status::PENDING.to_string(),
            build_url: String::new(),
            stderr: String::new(),
            created_at_ms: ts,
            updated_at_ms: ts,
        };
        let job_arc: BuildJobSlot = std::sync::Arc::new(tokio::sync::RwLock::new(job));
        {
            let mut map: tokio::sync::RwLockWriteGuard<'_, BuildJobMap> =
                BUILD_JOB_REGISTRY.write().await;
            map.insert(job_id, job_arc);
        }
        job_id
    }

    /// Publishes a build task message onto the build topic.
    ///
    /// The wire payload is
    /// `{job_id}{BUILD_TASK_SEPARATOR}{user_id}{BUILD_TASK_SEPARATOR}{project_id}{BUILD_TASK_SEPARATOR}{code}`
    /// so the worker can reconstruct every input without any extra
    /// round-trip to the controller.
    ///
    /// # Arguments
    ///
    /// - `BuildJobId` - The id returned by [`Self::register_pending_job`].
    /// - `i32` - The owning user id.
    /// - `i64` - The owning project id.
    /// - `&str` - The user-submitted Rust source.
    ///
    /// # Returns
    ///
    /// - `Result<(), String>`: Ok on successful publish, or an error from
    ///   the broker (typically a missing topic — only happens if the
    ///   message-queue bootstrap has not yet run).
    #[instrument_trace]
    pub async fn publish_build_task(
        job_id: BuildJobId,
        user_id: i32,
        project_id: i64,
        code: &str,
    ) -> Result<(), String> {
        let broker: &MessageQueueBroker = get_message_queue_broker();
        let payload: MessagePayload = format!(
            "{job_id}{BUILD_TASK_SEPARATOR}{user_id}{BUILD_TASK_SEPARATOR}{project_id}{BUILD_TASK_SEPARATOR}{code}"
        )
        .into_bytes();
        broker.publish(TOPIC_EUV_PLAYGROUND_BUILD, &payload).await
    }

    /// Looks up a build job by id, applying ownership authorization.
    ///
    /// # Arguments
    ///
    /// - `BuildJobId` - The job id from the URL.
    /// - `i32` - The authenticated user from the cookie; a
    ///   mismatch returns `None` so the controller can answer 404.
    ///
    /// # Returns
    ///
    /// - `Option<BuildJob>`: A clone of the row when it exists and the
    ///   supplied `user_id` owns it, otherwise `None`.
    #[instrument_trace]
    pub async fn get_build_status(job_id: BuildJobId, user_id: i32) -> Option<BuildJob> {
        let job_arc: BuildJobSlot = {
            let map: tokio::sync::RwLockReadGuard<'_, BuildJobMap> =
                BUILD_JOB_REGISTRY.read().await;
            match map.get(&job_id) {
                Some(j) => j.clone(),
                None => return None,
            }
        };
        let job: BuildJob = job_arc.read().await.clone();
        if job.user_id != user_id {
            return None;
        }
        Some(job)
    }

    /// Transitions a `pending` job into [`build_status::RUNNING`].
    ///
    /// Called by the build worker before spawning `wasm-pack`. No-op when
    /// the job id is not present (the controller may have purged it).
    ///
    /// # Arguments
    ///
    /// - `BuildJobId` - The job id.
    #[instrument_trace]
    pub async fn mark_job_running(job_id: BuildJobId) {
        let job_arc: Option<BuildJobSlot> = {
            let map: tokio::sync::RwLockReadGuard<'_, BuildJobMap> =
                BUILD_JOB_REGISTRY.read().await;
            map.get(&job_id).cloned()
        };
        let Some(job_arc) = job_arc else {
            return;
        };
        let mut job: tokio::sync::RwLockWriteGuard<'_, BuildJob> = job_arc.write().await;
        job.status = build_status::RUNNING.to_string();
        job.updated_at_ms = Self::now_ms();
    }

    /// Transitions a job into [`build_status::SUCCESS`] and stores the
    /// `build_url` the frontend should load.
    ///
    /// # Arguments
    ///
    /// - `BuildJobId` - The job id.
    /// - `&str` - The absolute path the frontend should load.
    #[instrument_trace]
    pub async fn mark_job_success(job_id: BuildJobId, build_url: &str) {
        let job_arc: Option<BuildJobSlot> = {
            let map: tokio::sync::RwLockReadGuard<'_, BuildJobMap> =
                BUILD_JOB_REGISTRY.read().await;
            map.get(&job_id).cloned()
        };
        let Some(job_arc) = job_arc else {
            return;
        };
        let mut job: tokio::sync::RwLockWriteGuard<'_, BuildJob> = job_arc.write().await;
        job.status = build_status::SUCCESS.to_string();
        job.build_url = build_url.to_string();
        job.stderr = String::new();
        job.updated_at_ms = Self::now_ms();
    }

    /// Transitions a job into [`build_status::FAILED`] and stores the
    /// captured stderr / error message.
    ///
    /// # Arguments
    ///
    /// - `BuildJobId` - The job id.
    /// - `&str` - Human-readable failure detail.
    #[instrument_trace]
    pub async fn mark_job_failed(job_id: BuildJobId, stderr: &str) {
        let job_arc: Option<BuildJobSlot> = {
            let map: tokio::sync::RwLockReadGuard<'_, BuildJobMap> =
                BUILD_JOB_REGISTRY.read().await;
            map.get(&job_id).cloned()
        };
        let Some(job_arc) = job_arc else {
            return;
        };
        let mut job: tokio::sync::RwLockWriteGuard<'_, BuildJob> = job_arc.write().await;
        job.status = build_status::FAILED.to_string();
        job.stderr = stderr.to_string();
        job.updated_at_ms = Self::now_ms();
    }

    /// Build-worker entry point.
    ///
    /// Parses the wire payload published by [`Self::publish_build_task`],
    /// runs `wasm-pack build`, and writes the final status back into the
    /// registry. Spawned by
    /// `bootstrap::application::euv_playground` for every received
    /// payload so the listener loop itself never blocks.
    ///
    /// # Arguments
    ///
    /// - `MessagePayload` - The raw bytes from the topic.
    #[instrument_trace]
    pub async fn run_build_for_job(payload: MessagePayload) {
        let sep_byte: u8 = BUILD_TASK_SEPARATOR.as_bytes()[0];
        let mut segments: Vec<&[u8]> = Vec::with_capacity(4);
        let mut start: usize = 0;
        for (idx, byte) in payload.iter().enumerate() {
            if *byte == sep_byte && segments.len() < 3 {
                segments.push(&payload[start..idx]);
                start = idx + 1;
            }
        }
        segments.push(&payload[start..]);
        let job_id_str: &[u8] = match segments.first() {
            Some(s) => s,
            None => {
                error!("Invalid build task payload: missing job id");
                return;
            }
        };
        let user_id_str: &[u8] = match segments.get(1) {
            Some(s) => s,
            None => {
                error!("Invalid build task payload: missing user id");
                return;
            }
        };
        let project_id_str: &[u8] = match segments.get(2) {
            Some(s) => s,
            None => {
                error!("Invalid build task payload: missing project id");
                return;
            }
        };
        let code_bytes: &[u8] = match segments.get(3) {
            Some(s) => s,
            None => {
                error!("Invalid build task payload: missing code");
                return;
            }
        };
        let job_id: BuildJobId = match std::str::from_utf8(job_id_str)
            .ok()
            .and_then(|s: &str| s.parse::<BuildJobId>().ok())
        {
            Some(v) => v,
            None => {
                error!("Invalid build task payload: job id is not a u64");
                return;
            }
        };
        let user_id: i32 = match std::str::from_utf8(user_id_str)
            .ok()
            .and_then(|s: &str| s.parse::<i32>().ok())
        {
            Some(v) => v,
            None => {
                error!("Invalid build task payload: user id is not i32");
                return;
            }
        };
        let project_id: i64 = match std::str::from_utf8(project_id_str)
            .ok()
            .and_then(|s: &str| s.parse::<i64>().ok())
        {
            Some(v) => v,
            None => {
                error!("Invalid build task payload: project id is not i64");
                return;
            }
        };
        let code: String = match String::from_utf8(code_bytes.to_vec()) {
            Ok(s) => s,
            Err(error) => {
                error!("Invalid build task payload: code is not UTF-8 {error}");
                Self::mark_job_failed(job_id, &format!("code is not UTF-8: {error}")).await;
                return;
            }
        };
        Self::mark_job_running(job_id).await;
        match Self::build_wasm_pack_output(&code, project_id).await {
            Ok(_target_dir) => {
                let build_url: String = format!(
                    "/static/euv-playground/tmp/{}/index.html",
                    Self::encode_id(project_id),
                );
                Self::mark_job_success(job_id, &build_url).await;
                info!(
                    "Euv playground build job {job_id} succeeded for user {user_id} project {project_id}"
                );
            }
            Err(stderr) => {
                Self::mark_job_failed(job_id, &stderr).await;
                warn!(
                    "Euv playground build job {job_id} failed for user {user_id} project {project_id}"
                );
            }
        }
    }

    /// Purges finished jobs whose `updated_at_ms` is older than
    /// [`BUILD_JOB_TTL_MS`] from [`BUILD_JOB_REGISTRY`].
    ///
    /// Spawned by
    /// `bootstrap::application::euv_playground` on a fixed cadence so
    /// abandoned clients cannot grow the registry without bound.
    #[instrument_trace]
    pub async fn purge_expired_jobs() {
        let now: i64 = Self::now_ms();
        let mut map: tokio::sync::RwLockWriteGuard<'_, BuildJobMap> =
            BUILD_JOB_REGISTRY.write().await;
        let before: usize = map.len();
        map.retain(|_, slot: &mut BuildJobSlot| {
            let snapshot: BuildJob = match slot.try_read() {
                Ok(g) => g.clone(),
                Err(_) => return true,
            };
            if snapshot.status != build_status::SUCCESS && snapshot.status != build_status::FAILED {
                return true;
            }
            now.saturating_sub(snapshot.updated_at_ms) < BUILD_JOB_TTL_MS
        });
        let removed: usize = before.saturating_sub(map.len());
        if removed > 0 {
            info!("Purged {removed} expired euv playground build job(s)");
        }
    }
}

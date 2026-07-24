use super::*;

/// One row in the `GET /api/euv/playground/projects` listing.
///
/// Fields mirror the on-disk `metadata.json` so the sidebar can render
/// without an extra GET per row.
#[derive(Clone, Data, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EuvPlaygroundProjectListItem {
    /// Stable project id (monotonic per-user counter).
    pub(super) id: String,
    /// Human-readable project name.
    pub(super) name: String,
    /// Last-modified time of the project (ms since unix epoch, UTC).
    /// Frontend can format this with `new Date(...)`.
    pub(super) updated_at_ms: i64,
    /// Code length in bytes (so the sidebar can show e.g. "1.4 KB" without
    /// an extra round-trip to fetch the source).
    pub(super) code_size: u64,
}

/// Response body for `GET /api/euv/playground/projects/{id}` — full project
/// (name + code + metadata). Returned to the editor when the user picks a
/// row from the sidebar.
#[derive(Clone, Data, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EuvPlaygroundProjectDetail {
    /// Stable project id.
    pub(super) id: String,
    /// Human-readable project name.
    pub(super) name: String,
    /// Current Rust source code.
    pub(super) code: String,
    /// Last-modified time (ms since unix epoch, UTC).
    pub(super) updated_at_ms: i64,
}

/// Response body for project mutation routes (create / save / delete) —
/// just returns the (possibly updated) project metadata so the frontend can
/// refresh the sidebar without an extra list call.
#[derive(Clone, Data, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EuvPlaygroundProjectMutationResponse {
    /// Project id of the affected row. For create = the newly-assigned id;
    /// for save / delete = the same id that came in.
    pub(super) id: String,
    /// Updated project name.
    pub(super) name: String,
    /// Updated last-modified time (ms since unix epoch, UTC).
    pub(super) updated_at_ms: i64,
    /// `true` if the project was deleted (only the delete route returns
    /// this in the data; other routes leave it false).
    pub(super) deleted: bool,
}

/// Response body for `GET /api/euv/playground/default-code` — the canonical
/// starter template the server uses both here and when creating a new
/// project on disk. The frontend reads this before opening a brand-new
/// editor so the template lives in one place.
#[derive(Clone, Data, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EuvPlaygroundDefaultCodeResponse {
    /// Default Rust source code pre-filled into a new project.
    pub(super) code: String,
}

/// Response body for `POST /api/euv/playground/run` — produced `index.html`
/// + glue JS + wasm bytes as base64-encoded JSON.
#[derive(Clone, Data, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EuvPlaygroundRunResponse {
    /// Whether the request to enqueue the build was accepted. The actual
    /// build runs asynchronously after this response is sent, so `ok`
    /// here means "queued", not "compiled".
    pub(super) ok: bool,
    /// Build job id assigned by the server, URL-encoded the same way
    /// order records encode their ids (`Encode::execute(CHARSETS, ...)`).
    /// The frontend uses this string directly as the path segment for
    /// `GET /api/euv/playground/run/status/{id}`.
    pub(super) job_id: String,
    /// Always `"pending"` at the moment this response is sent. The
    /// frontend can use the field without having to map the http code.
    pub(super) status: String,
    /// Empty on success. On `ok=false` (e.g. broker not initialized)
    /// this carries the human-readable error so the frontend can show
    /// it instead of a generic toast.
    pub(super) message: String,
    /// Reserved for the legacy in-process response shape — always empty
    /// in the async path. Kept so older clients that look for the key
    /// don't get `undefined` and crash.
    pub(super) html: String,
    /// See [`Self::html`].
    pub(super) js: String,
    /// See [`Self::html`].
    pub(super) wasm: String,
    /// See [`Self::html`].
    pub(super) stderr: String,
    /// Empty when `ok` is false. Populated by the status endpoint once
    /// the worker finishes, so the run response itself never carries it.
    pub(super) build_url: String,
}

/// Response body for `GET /api/euv/playground/run/status/{job_id}`.
///
/// Mirrors [`EuvPlaygroundRunResponse`] but adds the terminal-state
/// fields the frontend needs (`status`, `build_url`, `stderr`,
/// `updated_at_ms`). Both `job_id` and `project_id` are URL-encoded
/// strings so callers can copy them straight into the next request.
#[derive(Clone, Data, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EuvPlaygroundBuildStatusResponse {
    /// Build job id echoed back from the URL, URL-encoded for round-trip
    /// safety with the `POST /run` response.
    pub(super) job_id: String,
    /// Project id the job belongs to (URL-encoded; the sidebar can
    /// pass it straight to `GET /api/euv/playground/projects/get/{id}`
    /// without re-encoding).
    pub(super) project_id: String,
    /// One of `pending` / `running` / `success` / `failed`.
    pub(super) status: String,
    /// When `status == "success"`, the absolute path the frontend
    /// should load in its preview iframe. Empty otherwise.
    pub(super) build_url: String,
    /// When `status == "failed"`, the captured compile / linker output
    /// the frontend can render in its stderr pane. Empty otherwise.
    pub(super) stderr: String,
    /// Wall-clock timestamp (ms since unix epoch) the row was created.
    pub(super) created_at_ms: i64,
    /// Wall-clock timestamp (ms since unix epoch) of the last status
    /// transition. The frontend uses this to render a live "updated
    /// Xs ago" label while the job is still in flight.
    pub(super) updated_at_ms: i64,
}

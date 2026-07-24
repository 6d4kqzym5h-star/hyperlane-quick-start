use super::*;

/// Route — list the current user's playground projects (most-recent first).
#[route("/api/euv/playground/projects")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundProjectsListRoute;

/// Route — create a new playground project for the current user.
#[route("/api/euv/playground/projects/create")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundProjectsCreateRoute;

/// Route — read a project's full content (name + code + timestamps).
#[route("/api/euv/playground/projects/get/{id}")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundProjectsGetRoute;

/// Route — update an existing project's name and/or code.
#[route("/api/euv/playground/projects/save/{id}")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundProjectsSaveRoute;

/// Route — delete an existing project (irreversible).
#[route("/api/euv/playground/projects/delete/{id}")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundProjectsDeleteRoute;

/// Route — compile the current code of a project to wasm via `wasm-pack`.
///
/// Enqueues a build task on the message-queue topic and immediately
/// returns the new job id; the actual compile runs on a worker thread
/// started by `bootstrap::application::euv_playground`. Clients poll
/// the matching `BuildStatusRoute` to observe completion.
#[route("/api/euv/playground/run")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundRunRoute;

/// Route — read the status of a previously-enqueued build job.
///
/// Path segment is the `job_id` returned by [`EuvPlaygroundRunRoute`].
/// The job is looked up in the in-memory registry guarded by the
/// current user's id, so unauthorized callers get 404 instead of a
/// peek at someone else's build.
#[route("/api/euv/playground/run/status/{id}")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundBuildStatusRoute;

/// Route — read the default source code template that pre-fills brand-new
/// playground projects. The same string is reused on the server when a
/// project is created, so both `/create` and `/default-code` stay in sync.
#[route("/api/euv/playground/default-code")]
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundDefaultCodeRoute;

/// Zero-sized struct used purely as a namespace for euv-playground
/// controller-side helpers (cookie extraction, request validation, etc.).
/// Methods live in `impl.rs` under `impl EuvPlaygroundHelpers { ... }`.
#[derive(Clone, Copy, Default)]
pub struct EuvPlaygroundHelpers;

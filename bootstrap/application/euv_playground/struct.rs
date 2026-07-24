use super::*;

/// Bootstrap handler for the euv playground asynchronous build pipeline.
///
/// On startup it creates the build topic + consumer group, registers a
/// listener that drains every published payload into a worker
/// coroutine, and spawns a periodic GC task that purges finished jobs
/// older than `BUILD_JOB_TTL_MS` so abandoned clients cannot grow the
/// in-memory registry without bound.
#[derive(Clone, Copy, Data, Debug, Default)]
pub struct EuvPlaygroundBootstrap;

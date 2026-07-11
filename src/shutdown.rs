//! `SIGTERM` and `SIGINT`, for a harness whose real work runs somewhere else.
//!
//! The workload does not run in this process. It runs on the Docker daemon, in
//! another process tree, and `docker run` is only a client attached to it.
//! Killing the harness therefore does **not** kill the benchmark: the container
//! keeps running, orphaned, holding every core it was given. That is not a
//! cosmetic leak on a bench machine — an orphan burning ten cores is a bias in
//! whatever gets measured next, and nobody is left to notice.
//!
//! So shutdown here is not about draining in-flight work; it is about
//! *disowning* it. The container we interrupted has no valid sample to give —
//! a killed run is a wrong run, and a wrong run never enters the statistics.
//! The samples already on disk are the ones worth saving, and they are safe
//! before this module does anything: each is flushed as it is produced.
//!
//! The signal thread only sets a flag. Everything else polls it — the engine
//! between ticks of a wait, the runner between invocations. A harness that is
//! deliberately sequential has no business sharing a `Child` across threads to
//! save 100 ms of shutdown latency.

use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use anyhow::Result;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use tracing::{error, warn};

/// Set once, by the signal thread. Read everywhere that could block for long.
static ABORT: AtomicBool = AtomicBool::new(false);

/// How long a caller may take to notice. Short enough to stay well inside
/// Docker's ten-second grace period, which is what stands between a clean stop
/// and a `SIGKILL` that would orphan the container all over again.
pub const TICK: std::time::Duration = std::time::Duration::from_millis(100);

/// The campaign was interrupted by a signal.
///
/// Not a failure: the samples written before it are valid and renderable. It
/// travels as an `anyhow` error only because that is how the call stack unwinds
/// — `execute` recognises it by type and exits 0.
#[derive(Debug)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "interrupted by a signal")
    }
}

impl Error for Interrupted {}

/// Arm the handler. Idempotent in effect, but call it once, from `main`.
///
/// The second signal exits on the spot. A user who asks twice has told us the
/// polite path is taking too long, and the honest answer is to stop — at the
/// cost of the orphan this module exists to prevent, which is why the log line
/// says so out loud rather than exiting quietly.
///
/// "Twice" means twice *delivered*, not twice sent: `SIGTERM` and `SIGINT` are
/// not real-time signals, so the kernel merges a second one arriving while the
/// first is still pending. Two in the same instant are one. In practice the
/// escape hatch is for a shutdown that is visibly stuck — a `docker kill` that
/// hangs on an unresponsive daemon — and by then the first has long been
/// consumed.
pub fn install() -> Result<()> {
    let mut signals = Signals::new([SIGTERM, SIGINT])?;
    thread::spawn(move || {
        for signal in &mut signals {
            if ABORT.swap(true, Ordering::SeqCst) {
                error!(
                    signal,
                    "second signal: exiting now. The container in flight is left running — \
                     stop it with `docker kill` (its name starts with `langbench-`).",
                );
                std::process::exit(130);
            }
            warn!(
                signal,
                "shutting down: killing the container in flight and stopping the campaign. \
                 Samples already written are intact. Signal again to exit immediately.",
            );
        }
    });
    Ok(())
}

/// Has a signal arrived?
pub fn requested() -> bool {
    ABORT.load(Ordering::SeqCst)
}

/// `Err(Interrupted)` if a signal has arrived — the cooperative checkpoint.
///
/// Called where new work is about to start, never in the middle of a measured
/// invocation: stopping is refusing the next unit, not abandoning the current
/// one halfway and writing down what it half-said.
pub fn checkpoint() -> Result<()> {
    if requested() {
        return Err(Interrupted.into());
    }
    Ok(())
}

/// Did this error come from a signal, anywhere down the chain?
pub fn was_interrupted(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.is::<Interrupted>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_interruption_is_recognisable_through_a_context_chain() {
        // The engine adds context as it unwinds, so `execute` never sees the
        // bare error: it sees whatever the call stack wrapped it in.
        let error = anyhow::Error::from(Interrupted).context("running the container");
        assert!(was_interrupted(&error));
    }

    #[test]
    fn an_ordinary_failure_is_not_an_interruption() {
        let error = anyhow::anyhow!("`docker run` failed").context("measuring c-gcc");
        assert!(!was_interrupted(&error));
    }
}

//! Bounded validation scheduling after dependency admission.
use crate::{Runner, dispatch, timed_gate};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
pub(super) fn task(
    name: &str,
    root: &Path,
    runner: &Runner<'_>,
) -> Result<(), String> {
    eprintln!("Checking {name}...");
    let args: Vec<_> = name.split_whitespace().map(str::to_owned).collect();
    let mut proxy =
        |program: &str, argv: &[&str], cwd: &Path| runner(program, argv, cwd);
    timed_gate(
        name,
        || dispatch(&args, root, &mut proxy),
        |line| eprintln!("{line}"),
    )
}
pub(super) fn admission(
    root: &Path,
    runner: &Runner<'_>,
    dates: Option<&Path>,
) -> Result<(), String> {
    eprintln!("Checking dep-age-check...");
    timed_gate(
        "dep-age-check",
        || crate::dependencies::check(root, runner, dates),
        |line| eprintln!("{line}"),
    )
}
fn lane(
    tasks: &[&str],
    root: &Path,
    runner: &Runner<'_>,
    stop: &AtomicBool,
) -> Result<(), String> {
    for name in tasks {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if let Err(error) = task(name, root, runner) {
            stop.store(true, Ordering::Release);
            return Err(error);
        }
    }
    Ok(())
}
fn guarded_lane(
    tasks: &[&str],
    root: &Path,
    runner: &Runner<'_>,
    stop: &AtomicBool,
) -> Result<(), String> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        lane(tasks, root, runner, stop)
    }))
    .unwrap_or_else(|_| Err("validation worker panicked".into()));
    if result.is_err() {
        stop.store(true, Ordering::Release);
    }
    result
}
fn execute(
    root: &Path,
    runner: &Runner<'_>,
    dates: Option<&Path>,
) -> Result<(), String> {
    // Admission must complete before either lane compiles or executes inputs.
    admission(root, runner, dates)?;
    for name in ["canon-check", "fmt --check", "dupes"] {
        task(name, root, runner)?;
    }
    let stop = AtomicBool::new(false);
    let (rust, web) = std::thread::scope(|scope| {
        let rust = scope.spawn(|| {
            guarded_lane(&["clippy", "doc", "test"], root, runner, &stop)
        });
        let web = scope.spawn(|| {
            guarded_lane(
                &["deny", "frontend", "frontend-coverage", "frontend-dupes"],
                root,
                runner,
                &stop,
            )
        });
        // Join both lanes before observing errors; no validation work is detached.
        let rust = rust
            .join()
            .unwrap_or_else(|_| Err("validation worker panicked".into()));
        let web = web
            .join()
            .unwrap_or_else(|_| Err("validation worker panicked".into()));
        (rust, web)
    });
    rust?;
    web?;
    // Browser fixtures require both the fresh test binary and frontend output.
    // Keep filesystem-heavy fixture suites outside the Rust-test overlap;
    // preserve their tight contention/deadline contracts. Coverage has its
    // own build directory and runs after both lanes finish.
    for name in [
        "browser-test",
        "handoff-test",
        "bombyx-test",
        "coverage",
        "audit",
    ] {
        task(name, root, runner)?;
    }
    Ok(())
}
pub(super) fn parallel(
    root: &Path,
    runner: &Runner<'_>,
    dates: Option<&Path>,
) -> Result<(), String> {
    let started = Instant::now();
    let result = execute(root, runner, dates);
    eprintln!(
        "Validation wall timing: {}",
        serde_json::json!({"elapsed_ms":started.elapsed().as_millis(),"max_lanes":2,"outcome":if result.is_ok(){"pass"}else{"fail"}})
    );
    if result.is_ok() {
        println!("All required gates passed.");
    }
    result
}

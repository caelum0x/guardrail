//! Data-directory and run-report checks.

use crate::check::CheckResult;

/// The data directory exists (or can be created) and is writable.
pub fn check_data_dir(dir: &str) -> CheckResult {
    let name = format!("data dir: {dir}");
    if let Err(err) = std::fs::create_dir_all(dir) {
        return CheckResult::fail(name, format!("create failed: {err}"));
    }
    let probe = std::path::Path::new(dir).join(".guardrail-doctor-probe");
    if let Err(err) = std::fs::write(&probe, b"ok") {
        return CheckResult::fail(name, format!("write failed: {err}"));
    }
    if let Err(err) = std::fs::remove_file(&probe) {
        return CheckResult::fail(name, format!("cleanup failed: {err}"));
    }
    CheckResult::pass(name, "exists and writable")
}

/// The latest run report exists and parses (informational — absence is a warn,
/// since a fresh checkout has not run the agent yet).
pub fn check_run_report(path: &str) -> CheckResult {
    let name = format!("run report: {path}");
    if !std::path::Path::new(path).exists() {
        return CheckResult::warn(name, "absent (agent has not run yet)");
    }
    match std::fs::read_to_string(path).map(|raw| serde_json::from_str::<serde_json::Value>(&raw)) {
        Ok(Ok(value)) => {
            let detail = match value.get("cycles").and_then(|c| c.as_u64()) {
                Some(n) => format!("valid; {n} cycle(s) recorded"),
                None => "valid JSON".to_string(),
            };
            CheckResult::pass(name, detail)
        }
        Ok(Err(err)) => CheckResult::fail(name, format!("invalid JSON: {err}")),
        Err(err) => CheckResult::fail(name, format!("read failed: {err}")),
    }
}

/// The migrations directory exists and contains at least one `.sql` file.
pub fn check_migrations(dir: &str) -> CheckResult {
    let name = format!("migrations: {dir}");
    let read = match std::fs::read_dir(dir) {
        Ok(read) => read,
        Err(err) => return CheckResult::fail(name, format!("read failed: {err}")),
    };
    let sql = read
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
        .count();
    if sql >= 1 {
        CheckResult::pass(name, format!("{sql} migration file(s)"))
    } else {
        CheckResult::warn(name, "no .sql migrations found")
    }
}

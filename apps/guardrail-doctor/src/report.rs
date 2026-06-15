//! Checklist and JSON rendering of preflight results.

use crate::check::{CheckResult, Status};

struct Tally {
    passed: usize,
    warned: usize,
    failed: usize,
}

fn tally(results: &[CheckResult]) -> Tally {
    let failed = results.iter().filter(|r| r.status == Status::Fail).count();
    let warned = results.iter().filter(|r| r.status == Status::Warn).count();
    Tally {
        passed: results.len() - failed - warned,
        warned,
        failed,
    }
}

/// Print a checklist table and the final READY / NOT READY summary.
/// Returns true if every check passed (no failures).
pub fn report(results: &[CheckResult]) -> bool {
    let name_width = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max("CHECK".len());

    println!();
    println!("Guardrail Doctor \u{2014} Preflight Checks");
    println!("{}", "=".repeat(name_width + 30));
    println!("  {:<width$}  STATUS  DETAIL", "CHECK", width = name_width);
    println!("{}", "-".repeat(name_width + 30));

    for result in results {
        println!(
            "{} {:<width$}  {:<6}  {}",
            result.status.mark(),
            result.name,
            result.status.label(),
            result.detail,
            width = name_width,
        );
    }

    let t = tally(results);
    println!("{}", "-".repeat(name_width + 30));
    println!(
        "Summary: {} passed, {} warned, {} failed (of {} checks)",
        t.passed,
        t.warned,
        t.failed,
        results.len(),
    );

    let ready = t.failed == 0;
    println!("\n{}\n", if ready { "READY" } else { "NOT READY" });
    ready
}

/// Emit the checks as a JSON document (for CI / dashboards). Returns readiness.
pub fn report_json(results: &[CheckResult]) -> bool {
    let t = tally(results);
    let ready = t.failed == 0;

    let checks: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "status": r.status.json(),
                "detail": r.detail,
            })
        })
        .collect();

    let doc = serde_json::json!({
        "ready": ready,
        "summary": {
            "total": results.len(),
            "passed": t.passed,
            "warned": t.warned,
            "failed": t.failed,
        },
        "checks": checks,
    });
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
    ready
}

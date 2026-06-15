//! guardrail-doctor: preflight / readiness checks for the Guardrail stack.
//!
//! Runs a series of named checks (config loading, risk-policy validation,
//! eligible-asset universe, allowlist/universe consistency, Track-2 skills index
//! and ensemble weights, strategy presets, migrations, data-directory
//! writability, run report), prints a checklist (or `--json`), and exits with
//! code 1 if any check fails.

mod check;
mod checks;
mod report;

fn main() {
    let json = std::env::args().any(|a| a == "--json");
    let results = checks::run_checks();
    let ready = if json {
        report::report_json(&results)
    } else {
        report::report(&results)
    };
    if !ready {
        std::process::exit(1);
    }
}

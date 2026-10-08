// SPDX-License-Identifier: GPL-3.0-or-later
//! Chipload command line (Rust port, in progress; see docs/decisions/0004).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_job::load_job;
use clap::{Parser, Subcommand};
use serde_json::json;

const EXIT_FAILED: u8 = 1;

#[derive(Parser)]
#[command(name = "chipload", version, about = "2.5D CAM (Rust port in progress)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a G-code program against a job's stock, safe height and machine limits.
    Check {
        /// G-code file to check.
        gcode: PathBuf,
        /// Job file the program was generated from.
        #[arg(long)]
        job: PathBuf,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Check { gcode, job, json } => check(&gcode, &job, json),
    }
}

fn check(gcode: &Path, job_path: &Path, as_json: bool) -> ExitCode {
    let job = match load_job(job_path) {
        Ok(job) => job,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let text = match std::fs::read_to_string(gcode) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", gcode.display());
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let report = check_gcode(&text, &limits_from_job(&job));
    if as_json {
        let issues = |list: &[CheckIssue]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|i| json!({"line": i.line, "message": i.message}))
                .collect()
        };
        let out = json!({
            "ok": report.ok(),
            "errors": issues(&report.errors),
            "warnings": issues(&report.warnings),
        });
        println!("{out}");
    } else {
        for e in &report.errors {
            println!("error (line {}): {}", e.line, e.message);
        }
        for w in &report.warnings {
            println!("warning: {}", w.message);
        }
        println!(
            "{}",
            if report.ok() {
                "Check passed."
            } else {
                "Check FAILED."
            }
        );
    }
    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_FAILED)
    }
}

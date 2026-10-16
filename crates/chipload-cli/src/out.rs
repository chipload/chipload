// SPDX-License-Identifier: GPL-3.0-or-later
//! Where a command's output goes (docs/specs/scripting.md §3): printed as text, or, with
//! `--json`, collected into one JSON object printed when the command ends.

use std::fmt::Display;
use std::path::Path;
use std::process::ExitCode;

use chipload_job::Problem;
use serde_json::{Map, Value, json};

/// The version of the JSON output format; raised when a key changes meaning or goes.
pub const JSON_VERSION: i64 = 1;

pub const EXIT_FAILED: u8 = 1;

#[derive(Debug, Default)]
pub struct Out {
    json: bool,
    notes: Vec<String>,
    written: Vec<String>,
    fields: Map<String, Value>,
    error: Option<String>,
    problems: Vec<Problem>,
    /// Printed as it is instead of the envelope (`schema --json`).
    document: Option<Value>,
}

impl Out {
    pub fn new(json: bool) -> Out {
        Out {
            json,
            ..Out::default()
        }
    }

    pub fn json(&self) -> bool {
        self.json
    }

    /// A line of output: printed, or a note.
    pub fn line(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.json {
            self.notes.push(text);
        } else {
            println!("{text}");
        }
    }

    /// A line for stderr: printed there, or a note; "error: …" becomes the error.
    pub fn err(&mut self, text: impl Into<String>) {
        let text = text.into();
        if !self.json {
            eprintln!("{text}");
            return;
        }
        match text.strip_prefix("error: ") {
            Some(message) if self.error.is_none() => self.error = Some(message.to_string()),
            _ => self.notes.push(text),
        }
    }

    /// Report `err` and return the failure exit code.
    pub fn fail(&mut self, err: impl Display) -> ExitCode {
        self.err(format!("error: {err}"));
        ExitCode::from(EXIT_FAILED)
    }

    /// A file written: "Wrote PATH" (with `detail` after it), or its absolute path.
    pub fn wrote(&mut self, path: &Path, detail: &str) {
        if self.json {
            let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
            self.written.push(absolute.display().to_string());
        } else {
            println!("Wrote {}{detail}", path.display());
        }
    }

    /// A key of the command's own (JSON only; the text output says it in words).
    pub fn set(&mut self, key: &str, value: Value) {
        self.fields.insert(key.to_string(), value);
    }

    /// Every key of `object` as the command's own.
    pub fn set_all(&mut self, object: Value) {
        if let Value::Object(map) = object {
            self.fields.extend(map);
        }
    }

    /// The job's problems, with their paths, for the error.
    pub fn problems(&mut self, problems: Vec<Problem>) {
        self.problems = problems;
    }

    /// Text printed as it is (rendered G-code, the licences): to stdout, or under `key`.
    pub fn text(&mut self, key: &str, text: &str) {
        if self.json {
            self.fields.insert(key.to_string(), json!(text));
        } else {
            print!("{text}");
        }
    }

    /// A document printed as it is, also with `--json` (a schema).
    pub fn document(&mut self, value: Value) {
        self.document = Some(value);
    }

    /// End the command: print the JSON (the envelope and the command's keys), pass the
    /// exit code on.
    pub fn finish(self, code: ExitCode) -> ExitCode {
        if let Some(document) = self.document {
            match serde_json::to_string_pretty(&document) {
                Ok(text) => println!("{text}"),
                Err(err) => {
                    eprintln!("error: {err}");
                    return ExitCode::from(EXIT_FAILED);
                }
            }
            return code;
        }
        if !self.json {
            return code;
        }
        println!("{}", self.envelope(code));
        code
    }

    /// The JSON a `--json` command prints, instead of printing it (the MCP server): the
    /// document, or the envelope with the command's keys.
    pub fn into_json(self, code: ExitCode) -> Value {
        match self.document {
            Some(document) => document,
            None => self.envelope(code),
        }
    }

    /// The envelope (docs/specs/scripting.md §3) with the command's keys.
    fn envelope(self, code: ExitCode) -> Value {
        let ok = code == ExitCode::SUCCESS;
        let mut object = self.fields;
        object.insert("chipload_json".into(), json!(JSON_VERSION));
        object.insert("ok".into(), json!(ok));
        object.insert("notes".into(), json!(self.notes));
        object.insert("written".into(), json!(self.written));
        if !ok {
            let problems: Vec<Value> = self
                .problems
                .iter()
                .map(|p| json!({"path": p.path, "short": p.short, "text": p.text}))
                .collect();
            // A command that fails after a warning (update losing geometry) says why there.
            let message = self
                .error
                .or_else(|| self.notes.last().cloned())
                .unwrap_or_else(|| "the command failed".to_string());
            object.insert(
                "error".into(),
                json!({"message": message, "problems": problems}),
            );
        }
        Value::Object(object)
    }
}

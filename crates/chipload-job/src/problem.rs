// SPDX-License-Identifier: GPL-3.0-or-later
//! What is wrong with a project, and where (docs/specs/live-problems.md §4).

use std::fmt;

/// One problem: the field it is about, the message for that field, and the full message
/// as the CLI prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The field, as a path into the project file (`operations.1.feed`, `machine.safe_z`,
    /// `tools.0.rpm`), or a whole operation (`operations.1`).
    pub path: String,
    /// The message without the place: "must be greater than 0, got 0".
    pub short: String,
    /// The full message: "operations.1.feed: must be greater than 0, got 0".
    pub text: String,
}

impl Problem {
    pub fn new(path: impl Into<String>, short: impl Into<String>, text: impl Into<String>) -> Self {
        Problem {
            path: path.into(),
            short: short.into(),
            text: text.into(),
        }
    }

    /// A problem worded "path: message", as [`crate::Job::validate`] words them.
    pub fn from_path_text(text: &str) -> Self {
        match text.split_once(": ") {
            Some((path, short)) if !path.is_empty() && !path.contains(char::is_whitespace) => {
                Problem::new(path, short, text)
            }
            _ => Problem::new("", text, text),
        }
    }

    /// The operation (index in `operations`) the problem is about, if any.
    pub fn operation(&self) -> Option<usize> {
        let rest = self.path.strip_prefix("operations.")?;
        rest.split('.').next()?.parse().ok()
    }

    /// The field within its operation (`feed`, `tabs.width`), or the whole path for the
    /// rest of the project; empty for an operation as a whole.
    pub fn field(&self) -> &str {
        match self.operation() {
            Some(i) => {
                let prefix = format!("operations.{i}");
                let rest = self.path.get(prefix.len()..).unwrap_or("");
                rest.strip_prefix('.').unwrap_or(rest)
            }
            None => &self.path,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_split_into_operation_and_field() {
        let p = Problem::from_path_text("operations.12.tabs.width: must be greater than 0, got 0");
        assert_eq!(p.path, "operations.12.tabs.width");
        assert_eq!(p.short, "must be greater than 0, got 0");
        assert_eq!(p.operation(), Some(12));
        assert_eq!(p.field(), "tabs.width");
        let whole = Problem::new("operations.3", "no geometry", "Operation 4: no geometry");
        assert_eq!((whole.operation(), whole.field()), (Some(3), ""));
        let job = Problem::from_path_text("machine.safe_z: must be above 0");
        assert_eq!((job.operation(), job.field()), (None, "machine.safe_z"));
        let loose = Problem::from_path_text("Job has no operations");
        assert_eq!(
            (loose.path.as_str(), loose.short.as_str()),
            ("", "Job has no operations")
        );
        assert_eq!(loose.to_string(), "Job has no operations");
    }
}

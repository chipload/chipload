// SPDX-License-Identifier: GPL-3.0-or-later
//! What is wrong with a project, and where (docs/specs/live-problems.md §4).

use std::fmt;

use chipload_i18n::Message;

/// One problem: the field it is about, the message for that field, and the full message
/// as the CLI prints it. Each is kept as a `Message` too, which the app shows in its
/// language and units (docs/specs/translations.md §7); the strings are those messages in
/// English and mm, for the command line and `--json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The field, as a path into the project file (`operations.1.feed`, `machine.safe_z`,
    /// `tools.0.rpm`), or a whole operation (`operations.1`).
    pub path: String,
    /// The message without the place: "must be greater than 0, got 0".
    pub short: String,
    /// The full message: "operations.1.feed: must be greater than 0, got 0".
    pub text: String,
    /// `short` as a message.
    pub short_message: Message,
    /// `text` as a message.
    pub text_message: Message,
}

impl Problem {
    pub fn new(path: impl Into<String>, short: Message, text: Message) -> Self {
        Problem {
            path: path.into(),
            short: short.english(),
            text: text.english(),
            short_message: short,
            text_message: text,
        }
    }

    /// A problem whose full text is also its message for the field (nothing to shorten).
    pub fn whole(path: impl Into<String>, message: Message) -> Self {
        Problem::new(path, message.clone(), message)
    }

    /// A problem at `path` worded `short`, whose full text is "path: short".
    pub fn at(path: impl Into<String>, short: Message) -> Self {
        let path = path.into();
        let text = Message::new("job-problem-at")
            .arg("path", path.as_str())
            .arg("problem", short.clone());
        Problem::new(path, short, text)
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
        let short = Message::new("job-must-be-positive").arg("got", "0");
        let p = Problem::at("operations.12.tabs.width", short);
        assert_eq!(p.path, "operations.12.tabs.width");
        assert_eq!(p.short, "must be greater than 0, got 0");
        assert_eq!(
            p.text,
            "operations.12.tabs.width: must be greater than 0, got 0"
        );
        assert_eq!(p.operation(), Some(12));
        assert_eq!(p.field(), "tabs.width");
        let job = Problem::at(
            "machine.safe_z",
            Message::new("job-must-be-positive").arg("got", "0"),
        );
        assert_eq!((job.operation(), job.field()), (None, "machine.safe_z"));
        let whole = Problem::at(
            "operations.3",
            Message::new("job-must-be-positive").arg("got", "0"),
        );
        assert_eq!((whole.operation(), whole.field()), (Some(3), ""));
        let loose = Problem::whole("", Message::new("job-must-be-positive").arg("got", "0"));
        assert_eq!(
            (loose.path.as_str(), loose.short.as_str()),
            ("", "must be greater than 0, got 0")
        );
        assert_eq!(loose.to_string(), "must be greater than 0, got 0");
    }
}

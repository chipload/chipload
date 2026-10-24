// SPDX-License-Identifier: GPL-3.0-or-later
//! A warning for the user, and which operation it is about.

use std::fmt;
use std::ops::Deref;

use chipload_i18n::Message;

/// A warning from planning or checking a project: the text as shown, and the operation
/// (index in `operations`) it is about, so a list can lead to it without reading the text.
/// `text` is `message` in English and mm, for the command line and `--json`; the app shows
/// `message` in its language and units (docs/specs/translations.md §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub operation: Option<usize>,
    pub text: String,
    pub message: Message,
}

impl Note {
    /// A note about the operation at `operation`.
    pub fn at(operation: usize, message: Message) -> Self {
        Note {
            operation: Some(operation),
            text: message.english(),
            message,
        }
    }

    /// A note about the project as a whole.
    pub fn job(message: Message) -> Self {
        Note {
            operation: None,
            text: message.english(),
            message,
        }
    }
}

impl Deref for Note {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl PartialEq<str> for Note {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<&str> for Note {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

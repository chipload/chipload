// SPDX-License-Identifier: GPL-3.0-or-later
//! A warning for the user, and which operation it is about.

use std::fmt;
use std::ops::Deref;

/// A warning from planning or checking a project: the text as shown, and the operation
/// (index in `operations`) it is about, so a list can lead to it without reading the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub operation: Option<usize>,
    pub text: String,
}

impl Note {
    /// A note about the operation at `operation`.
    pub fn at(operation: usize, text: impl Into<String>) -> Self {
        Note {
            operation: Some(operation),
            text: text.into(),
        }
    }

    /// A note about the project as a whole.
    pub fn job(text: impl Into<String>) -> Self {
        Note {
            operation: None,
            text: text.into(),
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

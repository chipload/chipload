// SPDX-License-Identifier: GPL-3.0-or-later
//! A 3D model's file as the project holds it: base64 text, held once and shared by every
//! copy of the project, with what is read from it remembered beside it
//! (docs/specs/relief-wizard.md §2).

use std::any::Any;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A model's STL file, base64-encoded, as written in the project file. Cloning it shares
/// the text, so the window's undo steps and background plans don't copy a large file each
/// time; two copies compare equal at once when they share it.
///
/// What a reader makes from the text (the mesh) can be remembered with it
/// ([`ModelData::remembered`]): worked out once for the text, whichever copy asks first.
#[derive(Clone, Default)]
pub struct ModelData(Arc<Inner>);

#[derive(Default)]
struct Inner {
    text: String,
    read: OnceLock<Arc<dyn Any + Send + Sync>>,
}

impl ModelData {
    /// The file as base64 text.
    pub fn new(text: String) -> ModelData {
        ModelData(Arc::new(Inner {
            text,
            read: OnceLock::new(),
        }))
    }

    /// The base64 text.
    pub fn as_str(&self) -> &str {
        &self.0.text
    }

    pub fn is_empty(&self) -> bool {
        self.0.text.is_empty()
    }

    /// The text's length in bytes.
    pub fn len(&self) -> usize {
        self.0.text.len()
    }

    /// What `make` gives for the text, worked out once and remembered with it: a later
    /// call, from this copy or any that shares the text, gets the same value without
    /// `make`. Only one kind of value is remembered; asked for another kind, `make` runs
    /// each time. `make` must depend on the text alone, so remembering changes nothing
    /// but the time.
    pub fn remembered<T: Any + Send + Sync>(&self, make: impl FnOnce(&str) -> T) -> Arc<T> {
        let text = self.0.text.as_str();
        if let Some(kept) = self.0.read.get() {
            return match Arc::clone(kept).downcast::<T>() {
                Ok(value) => value,
                Err(_) => Arc::new(make(text)),
            };
        }
        // Two copies asking at once may both make it; the first kept wins, and both are
        // the same, since `make` depends on the text alone.
        let value = Arc::new(make(text));
        let shared: Arc<dyn Any + Send + Sync> = value.clone();
        let kept = self.0.read.get_or_init(|| shared);
        Arc::clone(kept).downcast::<T>().unwrap_or(value)
    }
}

impl From<String> for ModelData {
    fn from(text: String) -> Self {
        ModelData::new(text)
    }
}

impl PartialEq for ModelData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.text == other.0.text
    }
}

impl std::fmt::Debug for ModelData {
    // The length only: the text can be tens of megabytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ModelData({} bytes of base64)", self.0.text.len())
    }
}

impl Serialize for ModelData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.text)
    }
}

impl<'de> Deserialize<'de> for ModelData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(ModelData::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_share_the_text_and_what_was_read() {
        let data = ModelData::new("c29saWQ=".to_string());
        let copy = data.clone();
        assert!(Arc::ptr_eq(&data.0, &copy.0));
        assert_eq!(data, copy);
        let mut runs = 0;
        let first = data.remembered(|text| {
            runs += 1;
            text.len()
        });
        let again = copy.remembered(|_| -> usize { unreachable!() });
        assert_eq!((*first, *again, runs), (8, 8, 1));
        // Another kind isn't remembered: made each time.
        assert_eq!(*copy.remembered(|text| text.to_string()), "c29saWQ=");
    }

    #[test]
    fn equal_texts_are_equal_unshared_and_written_as_text() {
        let a = ModelData::new("abc".to_string());
        let b = ModelData::new("abc".to_string());
        assert_eq!(a, b);
        assert_ne!(a, ModelData::new("abd".to_string()));
        assert_eq!(serde_json::to_string(&a).unwrap(), "\"abc\"");
        let back: ModelData = serde_json::from_str("\"abc\"").unwrap();
        assert_eq!(back, a);
        assert_eq!(format!("{a:?}"), "ModelData(3 bytes of base64)");
    }
}

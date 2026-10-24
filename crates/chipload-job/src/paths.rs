// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading and changing a project's values by path (docs/specs/scripting.md §4): dot-separated
//! keys and array indexes from 0, as in the file and in problem paths
//! (`operations.1.end_depth`).

use chipload_i18n::Message;
use serde_json::Value;

use crate::{Job, JobError, parse_job};

/// Fields `set` refuses: saving writes them.
const FIXED: [&str; 2] = ["schema_version", "written_by"];

fn segments(path: &str) -> Result<Vec<&str>, String> {
    if path.is_empty() || path.split('.').any(str::is_empty) {
        return Err(format!(
            "\"{path}\" is not a path (keys and indexes joined by dots)"
        ));
    }
    Ok(path.split('.').collect())
}

/// The value at `path` in `root`.
pub fn get<'a>(root: &'a Value, path: &str) -> Result<&'a Value, String> {
    let mut here = root;
    for (k, key) in segments(path)?.into_iter().enumerate() {
        let next = match here {
            Value::Object(map) => map.get(key),
            Value::Array(items) => key.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        };
        here = next.ok_or_else(|| missing(path, k))?;
    }
    Ok(here)
}

fn missing(path: &str, k: usize) -> String {
    let upto: Vec<&str> = path.split('.').take(k + 1).collect();
    format!("{path}: there is no {}", upto.join("."))
}

/// Set the value at `path`. A missing key is added (loading decides whether the format has
/// it); an index one past the end of an array appends.
pub fn set(root: &mut Value, path: &str, value: Value) -> Result<(), String> {
    let keys = segments(path)?;
    if keys.len() == 1 && FIXED.contains(&keys[0]) {
        return Err(format!("{path} can't be set: saving writes it"));
    }
    let last = keys.len() - 1;
    let mut here = root;
    for (k, key) in keys.iter().enumerate() {
        here = match here {
            Value::Object(map) => {
                if k == last {
                    map.insert((*key).to_string(), value);
                    return Ok(());
                }
                map.get_mut(*key).ok_or_else(|| missing(path, k))?
            }
            Value::Array(items) => {
                let index = key
                    .parse::<usize>()
                    .map_err(|_| format!("{path}: \"{key}\" is not an index (0, 1, …)"))?;
                if k == last && index == items.len() {
                    items.push(value);
                    return Ok(());
                }
                let count = items.len();
                let item = items.get_mut(index).ok_or_else(|| {
                    format!("{path}: index {index} is past the end (there are {count})")
                })?;
                if k == last {
                    *item = value;
                    return Ok(());
                }
                item
            }
            _ => return Err(missing(path, k)),
        };
    }
    Ok(())
}

/// A value from the command line: JSON when it parses, else the text as a string.
pub fn parse_value(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

/// Split "path=value" at the first '='.
pub fn assignment(text: &str) -> Result<(String, Value), String> {
    match text.split_once('=') {
        Some((path, value)) => Ok((path.to_string(), parse_value(value))),
        None => Err(format!("\"{text}\" is not path=value")),
    }
}

/// The job with each value set, loaded and validated again as the current version; the
/// error names the first path that couldn't be set, or why the result doesn't load.
pub fn set_all(job: &Job, assignments: &[(String, Value)]) -> Result<Job, String> {
    let mut value = serde_json::to_value(job).map_err(|e| e.to_string())?;
    for (path, new) in assignments {
        set(&mut value, path, new.clone())?;
    }
    let shown: Vec<&str> = assignments.iter().map(|(p, _)| p.as_str()).collect();
    parse_job(value, "").map_err(|e| match e {
        JobError::Invalid { reason, .. } => Message::new("job-set-would-not-load")
            .arg("paths", shown.join(", "))
            .arg("reason", reason)
            .english(),
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paths_read_and_write_keys_and_indexes() {
        let mut v = json!({"a": {"b": [1, {"c": 2}]}});
        assert_eq!(get(&v, "a.b.1.c").unwrap(), &json!(2));
        assert_eq!(get(&v, "a.b.0").unwrap(), &json!(1));
        assert!(get(&v, "a.x").unwrap_err().contains("there is no a.x"));
        assert!(get(&v, "a.b.7").is_err());
        assert!(get(&v, "a..b").is_err());
        set(&mut v, "a.b.1.c", json!(5)).unwrap();
        set(&mut v, "a.b.2", json!("new")).unwrap();
        set(&mut v, "a.d", json!(null)).unwrap();
        assert_eq!(v, json!({"a": {"b": [1, {"c": 5}, "new"], "d": null}}));
        assert!(
            set(&mut v, "a.b.9", json!(0))
                .unwrap_err()
                .contains("past the end")
        );
        assert!(
            set(&mut v, "a.b.x", json!(0))
                .unwrap_err()
                .contains("not an index")
        );
        assert!(set(&mut v, "a.q.r", json!(0)).is_err());
        assert!(set(&mut v, "schema_version", json!(3)).is_err());
    }

    #[test]
    fn values_are_json_or_text() {
        assert_eq!(parse_value("6.5"), json!(6.5));
        assert_eq!(parse_value("null"), json!(null));
        assert_eq!(parse_value("{\"after_tool\": 1}"), json!({"after_tool": 1}));
        assert_eq!(parse_value("Outline"), json!("Outline"));
        assert_eq!(parse_value("\"6\""), json!("6"));
        assert_eq!(
            assignment("name=a=b").unwrap(),
            ("name".to_string(), json!("a=b"))
        );
        assert!(assignment("name").is_err());
    }
}

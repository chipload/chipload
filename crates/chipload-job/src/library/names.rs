// SPDX-License-Identifier: GPL-3.0-or-later
//! Uids and names: unique, compared without case.

use serde_json::Value;

/// The uid a library entry named `name` gets: FNV-1a 64 of the lower-cased name, as 16
/// lowercase hex digits (docs/specs/library-in-browser.md §2). Frozen: migrations of
/// projects and libraries both use it, and must agree forever.
pub fn uid_for_name(name: &str) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in name.to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

/// A uid not in `taken`: the uid of `name`, else of "name#2", "name#3" …
pub fn fresh_uid<'a>(name: &str, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    let mut uid = uid_for_name(name);
    let mut n = 2;
    while taken.clone().any(|t| t == uid) {
        uid = uid_for_name(&format!("{name}#{n}"));
        n += 1;
    }
    uid
}

/// 16 lowercase hex digits.
pub(crate) fn is_uid(uid: &str) -> bool {
    uid.len() == 16
        && uid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Every uid is well formed and unique in its list.
pub(crate) fn uid_problems<'a>(list: &str, uids: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut problems = Vec::new();
    let uids: Vec<&str> = uids.collect();
    for (i, uid) in uids.iter().enumerate() {
        if !is_uid(uid) {
            problems.push(format!(
                "{list}.{i}.uid: must be 16 lowercase hex digits, got \"{uid}\""
            ));
        } else if uids[..i].contains(uid) {
            problems.push(format!("{list}.{i}.uid: \"{uid}\" is used twice"));
        }
    }
    problems
}

/// Library v2 -> v3: each entry of `list` gets the uid of its name (one with a taken uid,
/// which only a hash collision gives, the next free one).
pub(super) fn add_uids(data: &mut serde_json::Map<String, Value>, list: &str) {
    if let Some(Value::Array(entries)) = data.get_mut(list) {
        let mut taken: Vec<String> = Vec::new();
        for entry in entries.iter_mut().filter_map(Value::as_object_mut) {
            let name = entry.get("name").and_then(Value::as_str).unwrap_or("");
            let uid = fresh_uid(name, taken.iter().map(String::as_str));
            taken.push(uid.clone());
            entry.insert("uid".into(), Value::from(uid));
        }
    }
    data.insert("schema_version".into(), Value::from(3));
}

/// Names compare ignoring case, so "6 mm Upcut" and "6 mm upcut" can't both exist.
pub fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

pub(crate) fn position<'a>(mut names: impl Iterator<Item = &'a str>, name: &str) -> Option<usize> {
    names.position(|n| same_name(n, name))
}

/// Names are not empty, trimmed and unique ignoring case; the default names an entry.
pub(crate) fn name_problems<'a>(
    list: &str,
    names: impl Iterator<Item = &'a str>,
    default: Option<&str>,
    default_field: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    let names: Vec<&str> = names.collect();
    for (i, name) in names.iter().enumerate() {
        if name.trim().is_empty() {
            problems.push(format!("{list}.{i}.name: must not be empty"));
        } else if name.trim() != *name {
            problems.push(format!(
                "{list}.{i}.name: must not start or end with spaces, got \"{name}\""
            ));
        }
        if names[..i].iter().any(|earlier| same_name(earlier, name)) {
            problems.push(format!(
                "{list}.{i}.name: \"{name}\" is used twice (names ignore case)"
            ));
        }
    }
    if let Some(default) = default
        && !names.iter().any(|n| same_name(n, default))
    {
        problems.push(format!("{default_field}: \"{default}\" is not in {list}"));
    }
    problems
}

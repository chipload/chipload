// SPDX-License-Identifier: GPL-3.0-or-later
//! The update notice (docs/specs/update-notice.md, ADR 0026): once a day, when the user
//! said yes, one HTTPS request for the latest published release on GitHub, compared with
//! this version. The request runs on a thread of its own; reading the answer and comparing
//! versions are plain functions, tested without a network. Nothing is downloaded or
//! installed, and nothing about the user is sent.

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

/// GitHub's newest published release: never a draft, never a pre-release.
const RELEASES_API: &str = "https://api.github.com/repos/chipload/chipload/releases/latest";
/// The releases page, opened when a release's own page isn't one of ours.
pub const RELEASES_PAGE: &str = "https://github.com/chipload/chipload/releases";
/// This version, as Cargo has it: "0.29.1".
pub const THIS_VERSION: &str = env!("CARGO_PKG_VERSION");
/// A day between daily checks, s.
const CHECK_EVERY_S: u64 = 24 * 60 * 60;
/// A request that takes longer is given up.
const TIMEOUT: Duration = Duration::from_secs(10);
/// The most of GitHub's answer that is read, bytes (an answer is a few KB).
const MAX_ANSWER_BYTES: u64 = 256 * 1024;

/// A version as three numbers, compared in order: 0.30.0 > 0.29.10 > 0.29.9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u64, u64, u64);

impl Version {
    /// "0.30.0" or "v0.30.0"; anything else (a pre-release "v0.30.0-rc1", "guide-0.29.1-1")
    /// is no version here.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut numbers = text.split('.').map(|n| {
            (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                .then(|| n.parse::<u64>().ok())
                .flatten()
        });
        let version = Version(numbers.next()??, numbers.next()??, numbers.next()??);
        numbers.next().is_none().then_some(version)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// The latest release: its version and the page to open for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub page: String,
}

/// GitHub's answer for the latest release: its `tag_name` and `html_url`. A page that
/// isn't one of this project's releases gives the releases page instead.
pub fn parse_answer(json: &str) -> Result<Release, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("not JSON: {err}"))?;
    let tag = value
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or("no tag_name")?;
    let version = Version::parse(tag).ok_or_else(|| format!("not a version: {tag}"))?;
    let page = value
        .get("html_url")
        .and_then(serde_json::Value::as_str)
        .filter(|url| url.starts_with("https://github.com/chipload/chipload/releases/"))
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    Ok(Release { version, page })
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A newer version than this one.
    Newer(Release),
    /// This version is the latest (or newer: a build from main).
    Latest,
    /// No answer, or one that couldn't be read: why.
    Failed(String),
}

/// What an answer (or a failed request) means for `this` version.
pub fn outcome(this: &str, answer: Result<String, String>) -> Outcome {
    let release = match answer.and_then(|json| parse_answer(&json)) {
        Ok(release) => release,
        Err(why) => return Outcome::Failed(why),
    };
    match Version::parse(this) {
        Some(this) if release.version > this => Outcome::Newer(release),
        _ => Outcome::Latest,
    }
}

/// Whether the daily check is due: the user said yes, and the last check is a day old or
/// more, or never ran, or is "in the future" (a clock set back).
pub fn due(check: Option<bool>, checked_at: Option<u64>, now: u64) -> bool {
    check == Some(true) && checked_at.is_none_or(|at| at > now || now - at >= CHECK_EVERY_S)
}

/// Now, seconds since 1970 (UTC); 0 if the clock is before that.
pub fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Ask GitHub for the latest release: the answer's text, or why there is none. Checked
/// against the system's own certificates (decision 6), through a proxy in `HTTPS_PROXY`
/// when one is set (ureq reads it), at most `TIMEOUT`.
pub fn fetch() -> Result<String, String> {
    let loaded = rustls_native_certs::load_native_certs();
    if loaded.certs.is_empty() {
        let why = loaded
            .errors
            .first()
            .map_or("none found".to_string(), ToString::to_string);
        return Err(format!("the system's certificates: {why}"));
    }
    let roots: Vec<ureq::tls::Certificate<'static>> = loaded
        .certs
        .iter()
        .map(|cert| ureq::tls::Certificate::from_der(cert.as_ref()).to_owned())
        .collect();
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::new_with_certs(&roots))
        .build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(TIMEOUT))
        .user_agent(format!("chipload/{THIS_VERSION}"))
        .build()
        .into();
    let mut response = agent
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|err| err.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_ANSWER_BYTES)
        .read_to_string()
        .map_err(|err| err.to_string())
}

/// The update notice's state in the window.
pub struct Updates {
    /// The user's answer (`update_check` in the settings): None until asked.
    pub check: Option<bool>,
    /// When the daily check last ran, s since 1970.
    pub checked_at: Option<u64>,
    /// A version the user chose to skip.
    pub skipped: Option<Version>,
    /// The question's bar closed without an answer, for this run.
    pub bar_closed: bool,
    /// A newer release than this one, from the last check.
    pub newer: Option<Release>,
    /// The daily check was considered this run (it runs at most once per start).
    pub started: bool,
    /// A check on its thread: its answer, and whether the user asked for it (Check now).
    pending: Option<(Receiver<Result<String, String>>, bool)>,
    /// The request: [`fetch`], or a stand-in in tests.
    pub fetch: fn() -> Result<String, String>,
}

impl Default for Updates {
    fn default() -> Self {
        Updates {
            check: None,
            checked_at: None,
            skipped: None,
            bar_closed: false,
            newer: None,
            started: false,
            pending: None,
            fetch,
        }
    }
}

impl Updates {
    /// The notice to show: a newer release that isn't skipped.
    pub fn notice(&self) -> Option<&Release> {
        self.newer
            .as_ref()
            .filter(|r| self.skipped != Some(r.version))
    }

    /// Whether a check is under way.
    pub fn checking(&self) -> bool {
        self.pending.is_some()
    }

    /// Start a check on a thread; `wake` is called once its answer is in. `manual`: the
    /// user asked (Help > Check for new versions now).
    pub fn start(&mut self, manual: bool, wake: impl FnOnce() + Send + 'static) {
        if self.pending.is_some() {
            return;
        }
        let (send, answer) = channel();
        let fetch = self.fetch;
        std::thread::spawn(move || {
            // The window may be gone by now: then nobody waits for the answer.
            let _ = send.send(fetch());
            wake();
        });
        self.pending = Some((answer, manual));
    }

    /// The finished check's outcome, and whether the user asked for it; None while it
    /// runs or when none was started.
    pub fn poll(&mut self) -> Option<(Outcome, bool)> {
        let (answer, manual) = self.pending.as_ref()?;
        let manual = *manual;
        let result = match answer.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("the check stopped".to_string()),
        };
        self.pending = None;
        let outcome = outcome(THIS_VERSION, result);
        if let Outcome::Newer(release) = &outcome {
            self.newer = Some(release.clone());
        }
        if outcome == Outcome::Latest {
            self.newer = None;
        }
        Some((outcome, manual))
    }

    /// Wait for the check under way (tests).
    #[cfg(test)]
    pub fn wait(&mut self) -> Option<(Outcome, bool)> {
        while self.pending.is_some() {
            if let Some(done) = self.poll() {
                return Some(done);
            }
            std::thread::yield_now();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn versions_compare_as_three_numbers() {
        assert!(v("0.30.0") > v("0.29.1"));
        assert!(v("v0.29.10") > v("0.29.9"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert_eq!(v("v0.29.1"), v("0.29.1"));
        assert_eq!(v("0.29.1").to_string(), "0.29.1");
        for tag in [
            "v0.30.0-rc1",
            "guide-0.29.1-1",
            "latest",
            "0.30",
            "0.30.0.1",
            "",
            "v",
            "0..1",
            "+1.0.0",
        ] {
            assert_eq!(Version::parse(tag), None, "{tag}");
        }
    }

    /// The shape of GitHub's answer (REST API: get the latest release), shortened.
    const ANSWER: &str = r#"{
        "url": "https://api.github.com/repos/chipload/chipload/releases/1",
        "html_url": "https://github.com/chipload/chipload/releases/tag/v0.30.0",
        "tag_name": "v0.30.0",
        "name": "Chipload 0.30.0",
        "draft": false,
        "prerelease": false,
        "assets": []
    }"#;

    #[test]
    fn githubs_answer_gives_the_version_and_its_page() {
        let release = parse_answer(ANSWER).unwrap();
        assert_eq!(release.version, v("0.30.0"));
        assert_eq!(
            release.page,
            "https://github.com/chipload/chipload/releases/tag/v0.30.0"
        );
        // A page elsewhere is never opened: the releases page instead.
        let foreign = ANSWER.replace("https://github.com/chipload", "https://example.com/x");
        assert_eq!(parse_answer(&foreign).unwrap().page, RELEASES_PAGE);
        assert!(parse_answer(&ANSWER.replace("tag_name", "tag")).is_err());
        assert!(parse_answer(&ANSWER.replace("v0.30.0\"", "nightly\"")).is_err());
        assert!(parse_answer("<html>").is_err());
    }

    #[test]
    fn an_answer_means_newer_latest_or_failed() {
        let answer = || Ok(ANSWER.to_string());
        assert!(matches!(outcome("0.29.1", answer()), Outcome::Newer(_)));
        assert_eq!(outcome("0.30.0", answer()), Outcome::Latest);
        assert_eq!(
            outcome("0.31.0", answer()),
            Outcome::Latest,
            "a build from main"
        );
        assert!(matches!(
            outcome("0.29.1", Err("offline".into())),
            Outcome::Failed(why) if why == "offline"
        ));
        assert!(matches!(
            outcome("0.29.1", Ok("{}".into())),
            Outcome::Failed(_)
        ));
    }

    #[test]
    fn the_daily_check_is_due_once_a_day_when_on() {
        let now = 1_800_000_000;
        assert!(!due(None, None, now), "not asked yet");
        assert!(!due(Some(false), None, now), "off");
        assert!(due(Some(true), None, now), "never checked");
        assert!(!due(Some(true), Some(now - 2 * 3600), now), "2 hours ago");
        assert!(due(Some(true), Some(now - 25 * 3600), now), "25 hours ago");
        assert!(due(Some(true), Some(now + 3600), now), "a clock set back");
    }

    /// The real request, by hand only (spec §8): `cargo test -p chipload-gui
    /// github_answers -- --ignored --nocapture`.
    #[test]
    #[ignore = "asks GitHub over the network"]
    fn github_answers() {
        let release = parse_answer(&fetch().unwrap()).unwrap();
        eprintln!("latest: {} at {}", release.version, release.page);
        assert!(release.version >= v("0.29.1"));
    }

    #[test]
    fn a_check_runs_on_a_thread_and_a_skipped_version_shows_nothing() {
        let mut updates = Updates {
            fetch: || Ok(ANSWER.to_string()),
            ..Updates::default()
        };
        updates.start(true, || {});
        assert!(updates.checking());
        let (outcome, manual) = updates.wait().unwrap();
        assert!(manual);
        assert!(matches!(outcome, Outcome::Newer(_)) == (v(THIS_VERSION) < v("0.30.0")));
        updates.newer = parse_answer(ANSWER).ok();
        assert!(updates.notice().is_some());
        updates.skipped = Some(v("0.30.0"));
        assert!(updates.notice().is_none());
        updates.newer = Some(Release {
            version: v("0.31.0"),
            page: RELEASES_PAGE.into(),
        });
        assert!(updates.notice().is_some(), "a later one shows");
    }
}

use crate::channel::ChannelState;

/// Kairos documentation (GitHub README until a dedicated docs site is ready).
pub const USER_DOCS_URL: &str = "https://github.com/BitConcepts/kairos#readme";

/// GitHub Issues page for the kairos terminal repo.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const KAIROS_ISSUES_URL: &str = "https://github.com/BitConcepts/kairos/issues";

/// GitHub Issues page for the specsmith AI governance repo.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const SPECSMITH_ISSUES_URL: &str = "https://github.com/BitConcepts/specsmith/issues";

/// Privacy policy placeholder — update when BitConcepts publishes one.
pub const PRIVACY_POLICY_URL: &str = "https://github.com/BitConcepts/kairos/blob/main/LICENSE";

/// Generate a pre-filled GitHub "New Issue" URL for the given repo.
///
/// `repo` should be one of `"kairos"` or `"specsmith"`.
///
/// The issue title and body will be pre-populated with the Kairos version
/// and OS information so reporters don't have to gather it manually.
pub fn report_bug_url(repo: &str) -> String {
    let base = format!(
        "https://github.com/BitConcepts/{repo}/issues/new"
    );
    let mut url = url::Url::parse(&base).expect("Should not fail to parse");
    let version = ChannelState::app_version().unwrap_or("dev");
    let os = os_info::get();
    let body = format!(
        "**Kairos version:** {version}\n\
         **OS:** {} {}\n\n\
         <!-- Describe the bug below -->",
        os.os_type(),
        os.version(),
    );
    url.query_pairs_mut()
        .append_pair("template", "bug_report.md")
        .append_pair("body", &body);
    url.to_string()
}

/// Legacy alias used by callers that previously opened the Warp feedback form.
/// Now routes to the kairos GitHub issue tracker.
pub fn feedback_form_url() -> String {
    report_bug_url("kairos")
}

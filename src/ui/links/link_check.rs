use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::domain::models::{CheckStatus, LinkCheck};

/// The translated name of a check status.
fn status_text(status: CheckStatus) -> String {
    match status {
        CheckStatus::Ok => t!("link-check-ok"),
        CheckStatus::Warning => t!("link-check-warning"),
        CheckStatus::Failed => t!("link-check-failed"),
    }
}

const fn status_class(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "check-ok",
        CheckStatus::Warning => "check-warning",
        CheckStatus::Failed => "check-failed",
    }
}

/// A small colored dot showing the result of a link check. Grey if the link was never checked.
#[component]
pub fn CheckDot(check: Option<LinkCheck>) -> Element {
    let (class, title) = match &check {
        None => ("check-never", t!("link-check-never")),
        Some(c) => (
            status_class(c.status),
            format!("{} – {}", status_text(c.status), c.describe()),
        ),
    };
    rsx! {
        span { class: "check-dot {class}", title: "{title}" }
    }
}

/// A line with a dot, the status text and the details of one check.
#[component]
pub fn CheckLine(check: LinkCheck) -> Element {
    let class = status_class(check.status);
    rsx! {
        p { class: "check-line {class}",
            span { class: "check-dot {class}" }
            "{status_text(check.status)} "
            span { class: "check-details", "({check.describe()})" }
        }
    }
}

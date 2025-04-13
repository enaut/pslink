use dioxus::prelude::*;
use pslink_shared::datatypes::Clicks;

#[component]
pub(crate) fn Stats(clicks: Clicks) -> Element {
    match clicks {
        Clicks::Count(count) => rsx! {
            div { "{count.number}" }
        },
        Clicks::Extended(stats) => rsx! {
            div { "{stats.total.number}" }
        },
    }
}

use dioxus::prelude::*;
use dioxus_i18n::t;
use pslink_shared::datatypes::{Clicks, Statistics, WeekCount};

#[component]
pub(crate) fn Stats(clicks: Clicks) -> Element {
    match clicks {
        Clicks::Count(count) => rsx! {
            div { "{count.number}" }
        },
        Clicks::Extended(stats) => match stats.values.iter().max().cloned() {
            Some(maximum) => {
                let insg_clicks = stats.total.number;
                rsx! {
                    div { padding_right: "10px",
                        Graph { stats, maximum }
                        span { style: "display: inline-block; font-size: 0.5em; background-color: rgba(255, 255, 255, 0.75); padding: 3px; border-radius: 14px; margin-left: -104px; position: relative; z-index: 1; width: auto; transform: translateY(-20px);",
                            {t!("total_clicks", count : insg_clicks)} // Text below the click statistic graph
                        }
                    }
                }
            }
            None => {
                let insg_clicks = stats.total.number;
                rsx! {
                    div { title: t!("tooltip_no_clicks", count : insg_clicks), "{insg_clicks}" } // Displayed as a tooltip when there have been no clicks on this link in the last 12 months.
                }
            }
        },
    }
}

#[component]
pub(crate) fn Graph(stats: Statistics, maximum: WeekCount) -> Element {
    let factor = 30.0 / f64::max(maximum.total.number as f64, 1.0);
    let full = stats.values;

    #[allow(clippy::cast_possible_truncation)]
    let normalized: Vec<i64> = full
        .iter()
        .map(|v| (30.0 - v.total.number as f64 * factor).round() as i64)
        .collect();

    // create svg points
    let mut points = Vec::new();
    points.push(format!("M 0 {}", &normalized[0]));
    #[allow(clippy::needless_range_loop)]
    for (i, value) in normalized.iter().enumerate() {
        points.push(format!("L {} {}", i * 2, value));
    }

    rsx! {
        svg { class: "statistics_graph", view_box: "0 0 104 30",
            path { d: points.join(" "), stroke: "green", fill: "transparent" }
        }
    }
}

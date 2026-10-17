//! Frontend UI components and shared Dioxus context.

pub mod home;
pub mod links;
pub mod login;
pub mod navbar;
pub mod translations;
pub mod users;

use dioxus::signals::Signal;
pub use links::Links;
pub use navbar::Navbar;

use crate::domain::{
    models::User,
    requests::{Operation, Ordering},
};

#[derive(Clone, Copy)]
pub struct PslinkContext {
    pub user: Signal<Option<User>>,
    pub hostname: Signal<String>,
    pub base_url: Signal<String>,
    pub brand_name: Signal<String>,
    pub is_demo: Signal<bool>,
}

impl Default for PslinkContext {
    fn default() -> Self {
        Self {
            user: Signal::new(None),
            hostname: Signal::new(String::new()),
            base_url: Signal::new(String::new()),
            brand_name: Signal::new("Pslink".to_string()),
            is_demo: Signal::new(false),
        }
    }
}

/// Toggle sort direction on a table column or switch to ascending order on a new column.
#[must_use]
pub fn toggle_column<C: PartialEq>(
    ordering: Option<Operation<C, Ordering>>,
    new_column: C,
) -> Option<Operation<C, Ordering>> {
    if let Some(Operation { column, value }) = ordering
        && column == new_column
    {
        let new_value = match value {
            Ordering::Ascending => Ordering::Descending,
            Ordering::Descending => Ordering::Ascending,
        };
        return Some(Operation {
            column,
            value: new_value,
        });
    }

    Some(Operation {
        column: new_column,
        value: Ordering::Ascending,
    })
}

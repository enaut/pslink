//! This crate contains all shared UI for the workspace.

pub mod home;
mod links;
pub mod login;
pub mod translations;
mod users;
use dioxus::signals::Signal;
pub use links::Links;

pub mod navbar;
pub use navbar::Navbar;
use pslink_shared::datatypes::User;
#[derive(Clone, Copy, Default)]
pub struct PslinkContext {
    pub user: Signal<Option<User>>,
}

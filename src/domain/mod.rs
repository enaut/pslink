//! Shared domain models, API request/response DTOs, and validation logic.

pub mod models;
pub mod requests;
pub mod validation;

use dioxus::prelude::{Asset, asset, manganis};

pub const BULMA_CSS: Asset = asset!("/assets/styling/bulma.css");

#[cfg(test)]
mod tests {
    use crate::domain::models::{Lang, Role, User};

    #[test]
    fn test_user_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<User>();
    }

    #[test]
    fn test_role_roundtrip() {
        for role in [
            Role::Disabled,
            Role::Regular,
            Role::Admin,
            Role::NotAuthenticated,
        ] {
            assert_eq!(Role::convert(role.to_i64()), role);
        }
    }

    #[test]
    fn test_lang_parsing() {
        use std::str::FromStr;
        assert_eq!(Lang::from_str("de-DE").unwrap(), Lang::DeDE);
        assert_eq!(Lang::from_str("de").unwrap(), Lang::DeDE);
        assert_eq!(Lang::from_str("en-US").unwrap(), Lang::EnUS);
        assert_eq!(Lang::from_str("en").unwrap(), Lang::EnUS);
    }
}

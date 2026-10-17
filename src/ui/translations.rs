use dioxus_i18n::{prelude::*, *};
use unic_langid::{LanguageIdentifier, langid};

#[must_use]
pub fn config(initial_language: LanguageIdentifier) -> I18nConfig {
    I18nConfig::new(initial_language)
        .with_locale((
            langid!("de-DE"),
            include_str!("../../translations/de-DE.ftl"),
        ))
        .with_locale((
            langid!("en-US"),
            include_str!("../../translations/en-US.ftl"),
        ))
        .with_fallback(langid!("en-US"))
}

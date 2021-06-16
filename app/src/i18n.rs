//! This modules contains the parts for making the app translatable.
use std::rc::Rc;

use fluent::{FluentArgs, FluentBundle, FluentResource};
use gloo_console::log;
use shared::datatypes::Lang;
use unic_langid::LanguageIdentifier;

/// A struct containing the data, functions and the current language to query the localized strings.
#[derive(Clone)]
pub struct I18n {
    lang: Lang,
    ftl_bundle: Rc<FluentBundle<FluentResource>>,
}

impl std::fmt::Debug for I18n {
    /// On debug print skip the bundle
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.lang)
    }
}

impl I18n {
    /// Create a new translator struct
    #[must_use]
    pub fn new(lang: Lang) -> Self {
        let ftl_bundle = Rc::new(Self::create_ftl_bundle(lang));
        Self { lang, ftl_bundle }
    }

    /// Get the current language
    #[must_use]
    pub const fn lang(&self) -> &Lang {
        &self.lang
    }

    /// Set the current language
    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
        self.ftl_bundle = Rc::new(Self::create_ftl_bundle(lang));
    }

    /// Get a localized string. Optionally with parameters provided in `args`.
    pub fn translate(&self, key: impl AsRef<str>, args: Option<&FluentArgs>) -> String {
        log!(key.as_ref());
        let msg = self
            .ftl_bundle
            .get_message(key.as_ref())
            .expect("Failed to get fluent message for key {}");

        let pattern = msg.value().expect("Failed to parse pattern");

        self.ftl_bundle
            .format_pattern(pattern, args, &mut vec![])
            .to_string()
    }
}

impl I18n {
    /// Prettyprint the language name
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self.lang {
            Lang::EnUS => "English (US)",
            Lang::DeDE => "Deutsch (Deutschland)",
        }
    }

    /// include the fluent messages into the binary
    #[must_use]
    pub const fn ftl_messages(lang: Lang) -> &'static str {
        macro_rules! include_ftl_messages {
            ( $lang_id:literal ) => {
                include_str!(concat!("../../locales/", $lang_id, "/main.ftl"))
            };
        }
        match lang {
            Lang::EnUS => include_ftl_messages!("en"),
            Lang::DeDE => include_ftl_messages!("de"),
        }
    }

    #[must_use]
    pub fn language_identifier(lang: Lang) -> LanguageIdentifier {
        lang.as_ref()
            .parse()
            .expect("parse Lang to LanguageIdentifier")
    }

    /// Create and initialize a fluent bundle.
    #[must_use]
    pub fn create_ftl_bundle(lang: Lang) -> FluentBundle<FluentResource> {
        let ftl_resource = FluentResource::try_new(Self::ftl_messages(lang).to_owned())
            .expect("parse FTL messages");

        let mut bundle = FluentBundle::new(vec![Self::language_identifier(lang)]);
        bundle.add_resource(ftl_resource).expect("add FTL resource");
        bundle
    }
}

use std::sync::Arc;

use fluent::{FluentArgs, FluentBundle, FluentResource};
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};
use unic_langid::LanguageIdentifier;

// A struct containing the functions and the current language to query the localized strings.
#[derive(Clone)]
pub struct I18n {
    lang: Lang,
    ftl_bundle: Arc<FluentBundle<FluentResource>>,
}

impl std::fmt::Debug for I18n {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.lang)
    }
}

impl I18n {
    /// Create a new translator struct
    #[must_use]
    pub fn new(lang: Lang) -> Self {
        Self {
            lang,
            ftl_bundle: Arc::new(lang.create_ftl_bundle()),
        }
    }

    /// Get the current language
    #[must_use]
    pub const fn lang(&self) -> &Lang {
        &self.lang
    }

    /// Set the current language
    pub fn set_lang(&mut self, lang: Lang) -> &Self {
        self.lang = lang;
        self.ftl_bundle = Arc::new(lang.create_ftl_bundle());
        self
    }

    /// Get a localized string. Optionally with parameters provided in `args`.
    pub fn translate(&self, key: impl AsRef<str>, args: Option<&FluentArgs>) -> String {
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

/// An `enum` containing the available languages.
/// To add an additional language add it to this enum aswell as an appropriate file into the locales folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, Display, EnumIter, EnumString, AsRefStr, Eq, PartialEq)]
pub enum Lang {
    #[strum(serialize = "en-US")]
    EnUS,
    #[strum(serialize = "de-DE")]
    DeDE,
}

impl Lang {
    /// Prettyprint the language name
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::EnUS => "English (US)",
            Self::DeDE => "Deutsch (Deutschland)",
        }
    }

    /// include the fluent messages into the binary
    #[must_use]
    pub const fn ftl_messages(self) -> &'static str {
        macro_rules! include_ftl_messages {
            ( $lang_id:literal ) => {
                include_str!(concat!("../locales/", $lang_id, "/main.ftl"))
            };
        }
        match self {
            Self::EnUS => include_ftl_messages!("en"),
            Self::DeDE => include_ftl_messages!("de"),
        }
    }

    #[must_use]
    pub fn to_language_identifier(self) -> LanguageIdentifier {
        self.as_ref()
            .parse()
            .expect("parse Lang to LanguageIdentifier")
    }

    /// Create and initialize a fluent bundle.
    #[must_use]
    pub fn create_ftl_bundle(self) -> FluentBundle<FluentResource> {
        let ftl_resource =
            FluentResource::try_new(self.ftl_messages().to_owned()).expect("parse FTL messages");

        let mut bundle = FluentBundle::new(vec![self.to_language_identifier()]);
        bundle.add_resource(ftl_resource).expect("add FTL resource");
        bundle
    }
}


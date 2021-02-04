use serde::Deserialize;
#[derive(Deserialize)]
pub(crate) struct LinkForm {
    pub title: String,
    pub target: String,
    pub code: String,
}

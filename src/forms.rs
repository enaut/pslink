use serde::Deserialize;
#[derive(Deserialize, Debug)]
pub(crate) struct LinkForm {
    pub title: String,
    pub target: String,
    pub code: String,
}

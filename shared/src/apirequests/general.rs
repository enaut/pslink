use std::ops::Deref;

use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct Filter {
    pub sieve: String,
}

impl Deref for Filter {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.sieve
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Ordering {
    Ascending,
    Descending,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Operation<T, V> {
    pub column: T,
    pub value: V,
}

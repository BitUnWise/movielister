use iddqd::{IdHashItem, id_upcast};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize, Debug)]
pub(crate) struct Movie {
    pub(crate) id: u32,
    pub(crate) name: String,
}

impl IdHashItem for Movie {
    type Key<'a> = u32;

    fn key(&self) -> Self::Key<'_> {
        self.id
    }

    id_upcast!();
}

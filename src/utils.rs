use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Guid {
    id: u32,
}

impl Guid {
    fn new(id: u32) -> Guid {
        Guid { id }
    }
}

pub struct GuidGenerator {
    used: HashSet<u32>,
}

impl GuidGenerator {
    pub fn new() -> GuidGenerator {
        GuidGenerator {
            used: HashSet::new(),
        }
    }

    pub fn generate(&mut self) -> Guid {
        let mut id = rand::random::<u32>();
        while self.used.contains(&id) {
            id = rand::random::<u32>();
        }
        self.used.insert(id);
        Guid::new(id)
    }
}

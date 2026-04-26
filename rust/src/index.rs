use std::collections::HashMap;

#[derive(Clone)]
pub struct Key {
    pub server: String,
    pub id: u32,
}

#[derive(Clone, Copy)]
pub struct Value {
    pub start: u64,
    pub size: u64,
}

struct Record {
    value: Option<Value>,
    used: bool,
}

pub struct Index {
    index: HashMap<String, HashMap<u32, Record>>,
}

impl Index {
    pub fn new() -> Self {
        Index {
            index: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: Key, value: Value) {
        self.index
            .entry(key.server)
            .or_default()
            .entry(key.id)
            .or_insert(Record {
                value: Some(value),
                used: false,
            });
    }

    pub fn get(&self, key: &Key) -> Option<Value> {
        self.index.get(&key.server)?.get(&key.id)?.value
    }

    pub fn use_value(&mut self, key: Key) -> bool {
        let entry = self.index.entry(key.server).or_default().entry(key.id);

        match entry {
            // Add record and mark it as used
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(Record {
                    value: None,
                    used: true,
                });
                false
            }
            // Return whether it was used and set it as true
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let used = e.get().used;
                e.get_mut().used = true;
                used
            }
        }
    }

    pub fn iter_unused(&mut self) -> impl Iterator<Item = Value> {
        self.index.values_mut().flat_map(|map| {
            map.values_mut()
                .filter(|Record { value: _, used }| !used)
                .map(|Record { value, used }| {
                    *used = true;
                    // Using unwrap as Records that haven't been used can never have None for value
                    (*value).unwrap()
                })
        })
    }
}

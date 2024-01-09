use std::{
    collections::{BinaryHeap, HashMap},
    hash::Hash,
};

pub trait ItemSize {
    fn size(&self) -> usize;
}

struct CacheEntry<Key> {
    key: Key,
    frequency: usize,
    priority: usize,
}

pub struct LfuCache<Key: Hash + Eq, Item: ItemSize> {
    size_in_bytes: usize,
    max_size_in_bytes: usize,
    key_val: HashMap<Key, (Item, usize, usize)>,
    heap: BinaryHeap<CacheEntry<Key>>,
}

impl ItemSize for Vec<u8> {
    fn size(&self) -> usize {
        self.len() * std::mem::size_of::<u8>()
    }
}

impl<Key> Ord for CacheEntry<Key> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        //Compare first using priority and after that using frequency
        self.priority
            .cmp(&other.priority)
            .then(self.frequency.cmp(&other.frequency))
    }
}

impl<Key> PartialOrd for CacheEntry<Key> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Key> Eq for CacheEntry<Key> {}

impl<Key> PartialEq for CacheEntry<Key> {
    fn eq(&self, other: &Self) -> bool {
        //Compare first using priority and after that using frequency
        self.priority == other.priority && self.frequency == other.frequency
    }
}

impl<Key: Hash + Eq + Clone, Item: ItemSize> LfuCache<Key, Item> {
    pub fn new(capacity: usize) -> Self {
        Self {
            size_in_bytes: 0,
            max_size_in_bytes: capacity,
            key_val: HashMap::new(),
            heap: BinaryHeap::new(),
        }
    }

    pub fn insert(&mut self, key: &Key, value: Item, priority: usize) {
        let size = value.size() + std::mem::size_of::<CacheEntry<Key>>();

        if size > self.max_size_in_bytes {
            panic!("Item is too large to fit in cache");
        }
        self.size_in_bytes += size;

        self.shrink_to_fit(self.max_size_in_bytes);

        self.key_val.insert(key.clone(), (value, 0, priority));

        let entry = CacheEntry::<Key> {
            key: key.clone(),
            frequency: 0,
            priority,
        };

        self.heap.push(entry);
    }

    pub fn shrink_to_fit(&mut self, max_size: usize) {
        self.max_size_in_bytes = max_size;
        while self.size_in_bytes > self.max_size_in_bytes {
            let entry = self.heap.pop().unwrap();
            self.size_in_bytes -= std::mem::size_of::<CacheEntry<Key>>();

            if entry.frequency != self.key_val[&entry.key].1
                || entry.priority != self.key_val[&entry.key].2
            {
                continue;
            }

            self.size_in_bytes -= self.key_val[&entry.key].0.size();
            self.key_val.remove(&entry.key);
        }
    }

    pub fn get(&mut self, key: &Key) -> Option<&Item> {
        if let Some((item, frequency, priority)) = self.key_val.get_mut(key) {
            if let Some(result) = frequency.checked_add(1) {
                *frequency = result;
                self.heap.push(CacheEntry {
                    key: key.clone(),
                    frequency: *frequency,
                    priority: *priority,
                });
            }

            Some(item)
        } else {
            None
        }
    }
}

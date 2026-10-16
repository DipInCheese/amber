// SPDX-License-Identifier: GPL-3.0-or-later
//! A small in-memory LRU for converted photo renditions (see `heic.rs`).
//! Converting a HEIC takes a good fraction of a second; without this,
//! scrolling back past a photo - or just re-rendering a row - would redo it
//! every time and make a media-heavy conversation stutter.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub struct RenditionCache {
    entries: HashMap<String, Arc<Vec<u8>>>,
    /// Keys from least to most recently used.
    order: VecDeque<String>,
    bytes: usize,
    capacity: usize,
}

impl RenditionCache {
    #[must_use]
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            capacity: capacity_bytes,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Arc<Vec<u8>>> {
        let value = self.entries.get(key)?.clone();
        self.touch(key);
        Some(value)
    }

    pub fn insert(&mut self, key: String, value: Arc<Vec<u8>>) {
        // One oversized item would just flush everything else for nothing.
        if value.len() > self.capacity {
            return;
        }
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes -= previous.len();
            self.order.retain(|existing| existing != &key);
        }
        self.bytes += value.len();
        self.order.push_back(key.clone());
        self.entries.insert(key, value);

        while self.bytes > self.capacity {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&oldest) {
                self.bytes -= evicted.len();
            }
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(position) = self.order.iter().position(|existing| existing == key) {
            if let Some(entry) = self.order.remove(position) {
                self.order.push_back(entry);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(len: usize) -> Arc<Vec<u8>> {
        Arc::new(vec![0; len])
    }

    #[test]
    fn returns_what_was_inserted() {
        let mut cache = RenditionCache::new(100);
        cache.insert("a".into(), blob(10));
        assert_eq!(cache.get("a").unwrap().len(), 10);
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn evicts_the_least_recently_used_when_full() {
        let mut cache = RenditionCache::new(30);
        cache.insert("a".into(), blob(10));
        cache.insert("b".into(), blob(10));
        cache.insert("c".into(), blob(10));
        // Using "a" makes "b" the oldest.
        assert!(cache.get("a").is_some());

        cache.insert("d".into(), blob(10));

        assert!(cache.get("b").is_none());
        assert!(cache.get("a").is_some());
        assert!(cache.get("c").is_some());
        assert!(cache.get("d").is_some());
    }

    #[test]
    fn replacing_a_key_doesnt_double_count_its_size() {
        let mut cache = RenditionCache::new(25);
        cache.insert("a".into(), blob(20));
        cache.insert("a".into(), blob(20));
        cache.insert("b".into(), blob(5));
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_some());
    }

    #[test]
    fn ignores_an_item_bigger_than_the_whole_cache() {
        let mut cache = RenditionCache::new(10);
        cache.insert("small".into(), blob(5));
        cache.insert("huge".into(), blob(50));
        assert!(cache.get("huge").is_none());
        assert!(cache.get("small").is_some());
    }
}

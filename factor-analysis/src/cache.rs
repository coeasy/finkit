use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Full research artifact identity; prevents unsafe cache reuse across semantic/data revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResearchArtifactKey {
    pub data_revision: u64,
    pub data_fingerprint: u64,
    pub plan_fingerprint: u64,
    pub semantics_fingerprint: u64,
    pub calendar_fingerprint: u64,
    pub universe_fingerprint: u64,
}

/// Small deterministic FIFO revision cache used above the core scratch `BufferArena`.
#[derive(Debug, Clone)]
pub struct ResearchCache<V> {
    capacity: usize,
    values: BTreeMap<ResearchArtifactKey, V>,
    order: VecDeque<ResearchArtifactKey>,
}

impl<V> ResearchCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), values: BTreeMap::new(), order: VecDeque::new() }
    }

    pub fn get(&self, key: &ResearchArtifactKey) -> Option<&V> { self.values.get(key) }

    pub fn insert(&mut self, key: ResearchArtifactKey, value: V) {
        if self.values.contains_key(&key) {
            self.values.insert(key, value);
            return;
        }
        while self.values.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() { self.values.remove(&oldest); }
            else { break; }
        }
        self.order.push_back(key);
        self.values.insert(key, value);
    }

    pub fn invalidate_revision(&mut self, revision: u64) {
        self.values.retain(|key, _| key.data_revision != revision);
        self.order.retain(|key| key.data_revision != revision);
    }

    pub fn len(&self) -> usize { self.values.len() }
    pub fn is_empty(&self) -> bool { self.values.is_empty() }
}

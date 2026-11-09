//! [`LockedReplica`]：几条连接同时进来时，对存储的读写一次一个。只在单次调用内持锁，传输时不占着。

use std::sync::{Arc, Mutex};

use super::lock;
use crate::error::SyncError;
use crate::record::{Record, Stamp};
use crate::sync::Replica;

#[derive(Clone)]
pub(super) struct LockedReplica(Arc<Mutex<Box<dyn Replica + Send>>>);

impl LockedReplica {
    pub fn new(replica: Box<dyn Replica + Send>) -> Self {
        Self(Arc::new(Mutex::new(replica)))
    }
}

impl Replica for LockedReplica {
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError> {
        lock(&self.0).stamps()
    }

    fn record(&self, id: &str) -> Result<Option<Record>, SyncError> {
        lock(&self.0).record(id)
    }

    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        lock(&self.0).apply(record)
    }
}

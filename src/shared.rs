use std::{cmp::Reverse, collections::BinaryHeap, sync::Arc};

pub type PlayerId = u32;

pub struct LowestIdManager(Arc<tokio::sync::Mutex<LowestIdManagerInner>>);

#[must_use = "AsyncIdGuard must be explicitly released with .release().await"]
pub struct IdGuard(LowestIdManager, PlayerId);

impl Clone for LowestIdManager {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

struct LowestIdManagerInner {
    heap: BinaryHeap<Reverse<PlayerId>>,
    next_id: PlayerId,
}

impl LowestIdManager {
    pub fn new() -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(LowestIdManagerInner {
            heap: BinaryHeap::new(),
            next_id: 1,
        })))
    }

    pub async fn acquire_id(&self) -> anyhow::Result<IdGuard> {
        let mut writer = self.0.lock().await;
        if let Some(Reverse(id)) = writer.heap.pop() {
            return Ok(IdGuard(self.clone(), id));
        }

        let id = writer.next_id;

        // stop giving id's since we are full
        if id >= PlayerId::MAX {
            return Err(anyhow::anyhow!("Id's are full"));
        }

        writer.next_id += 1;
        return Ok(IdGuard(self.clone(), id));
    }

    pub async fn release(&self, id: PlayerId) {
        let mut writer = self.0.lock().await;
        writer.heap.push(Reverse(id));
    }
}

impl IdGuard {
    pub async fn release(&mut self) {
        self.0.release(self.1).await;
    }

    pub fn inner(&self) -> PlayerId {
        return self.1;
    }
}

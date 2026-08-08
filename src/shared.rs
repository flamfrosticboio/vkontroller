// Vkontroller - Turns your browser into a virtual game controller
// Copyright (C) 2026  flamfrosticboio
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
        if id == PlayerId::MAX {
            return Err(anyhow::anyhow!("Id's are full"));
        }

        writer.next_id += 1;

        Ok(IdGuard(self.clone(), id))
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
        self.1
    }
}

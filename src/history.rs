use crate::storage::Storage;
use std::io;
use std::collections::VecDeque;

pub const MAX_BYTES: usize = 256 * 1024;
const MAX_ENTRIES: usize = 200;

#[derive(Default)]
pub struct History {
    entries: VecDeque<(u64, String, bool)>,
    pub paused: bool,
    next_id: u64,
    storage: Option<Storage>,
}

impl History {
    pub fn persistent(storage: Storage) -> Self {
        Self { storage: Some(storage), ..Self::default() }
    }

    pub fn capture(&mut self, text: String) -> io::Result<()> {
        if self.paused || text.is_empty() || text.len() > MAX_BYTES {
            return Ok(());
        }

        if let Some(storage) = &mut self.storage {
            return storage.capture(&text).map_err(io::Error::other);
        }

        let entry = match self.entries.iter().position(|(_, existing, _)| existing == &text) {
            Some(index) => self.entries.remove(index).unwrap(),
            None => {
                self.next_id += 1;
                (self.next_id, text, false)
            }
        };

        self.entries.push_front(entry);
        self.prune_memory();
        Ok(())
    }

    pub fn entries(&self) -> io::Result<Vec<(u64, String, bool)>> {
        match &self.storage {
            Some(storage) => storage.entries().map_err(io::Error::other),
            None => Ok(self.entries.iter().cloned().collect()),
        }
    }

    pub fn get(&self, id: u64) -> io::Result<Option<String>> {
        match &self.storage {
            Some(storage) => storage.get(id).map_err(io::Error::other),
            None => Ok(self.entries.iter().find(|(entry_id, _, _)| *entry_id == id)
                .map(|(_, text, _)| text.clone())),
        }
    }

    pub fn remove(&mut self, id: u64) -> io::Result<()> {
        if let Some(storage) = &self.storage {
            return storage.remove(id).map_err(io::Error::other);
        }
        self.entries.retain(|(entry_id, _, _)| *entry_id != id);
        Ok(())
    }

    pub fn clear(&mut self) -> io::Result<()> {
        if let Some(storage) = &self.storage {
            return storage.clear().map_err(io::Error::other);
        }
        self.entries.clear();
        Ok(())
    }

    pub fn pin(&mut self, id: u64, pinned: bool) -> io::Result<bool> {
        if let Some(storage) = &mut self.storage {
            return storage.pin(id, pinned).map_err(io::Error::other);
        }
        let Some(entry) = self.entries.iter_mut().find(|(entry_id, _, _)| *entry_id == id) else {
            return Ok(false);
        };
        entry.2 = pinned;
        self.prune_memory();
        Ok(true)
    }

    fn prune_memory(&mut self) {
        let mut remaining = MAX_ENTRIES;
        self.entries.retain(|(_, _, pinned)| {
            if *pinned {
                return true;
            }
            if remaining == 0 {
                return false;
            }
            remaining -= 1;
            true
        });
    }

    pub fn len(&self) -> io::Result<usize> {
        match &self.storage {
            Some(storage) => storage.len().map_err(io::Error::other),
            None => Ok(self.entries.len()),
        }
    }
}

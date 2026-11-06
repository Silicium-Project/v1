use crate::{
    FsyncPolicy, StoreConfig, StoreError,
    record::{MAX_RECORD_BYTES, frame},
    segment::{list, path_of, scan},
};
use sil_ledger::Event;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
};

/// Ce que l'ouverture a lu et réparé.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenReport {
    pub events_read: u64,
    pub segments: u64,
    /// Un enregistrement interrompu a été tronqué en fin de dernier segment.
    pub repaired_tail: bool,
}

/// Écrivain du journal sur disque.
pub struct EventStore {
    config: StoreConfig,
    file: File,
    number: u64,
    size: u64,
    unsynced: u32,
}

impl EventStore {
    /// Ouvre (ou crée) le dossier, rejoue chaque événement via `on_event` dans l'ordre
    /// d'écriture, répare une fin interrompue, puis ouvre le segment courant en ajout.
    pub fn open(
        config: StoreConfig,
        mut on_event: impl FnMut(Event),
    ) -> Result<(Self, OpenReport), StoreError> {
        fs::create_dir_all(&config.dir)?;
        let numbers = list(&config.dir)?;
        let mut report = OpenReport::default();
        let mut tail = (1u64, 0u64);
        for (index, number) in numbers.iter().copied().enumerate() {
            let path = path_of(&config.dir, number);
            let last = index + 1 == numbers.len();
            let result = scan(&path, last, |payload| {
                let event = Event::from_bytes(payload, MAX_RECORD_BYTES)?;
                on_event(event);
                report.events_read += 1;
                Ok(())
            })?;
            report.segments += 1;
            if result.torn {
                OpenOptions::new()
                    .write(true)
                    .open(&path)?
                    .set_len(result.good_len)?;
                report.repaired_tail = true;
            }
            tail = (number, result.good_len);
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path_of(&config.dir, tail.0))?;
        let store = Self {
            config,
            file,
            number: tail.0,
            size: tail.1,
            unsynced: 0,
        };
        Ok((store, report))
    }

    /// Ajoute un événement (appel après son acceptation par le journal).
    pub fn append(&mut self, event: &Event) -> Result<(), StoreError> {
        let record = frame(&event.to_bytes()?);
        if self.size > 0 && self.size + record.len() as u64 > self.config.max_segment_bytes {
            self.rotate()?;
        }
        self.file.write_all(&record)?;
        self.size += record.len() as u64;
        self.unsynced += 1;
        match self.config.fsync {
            FsyncPolicy::Always => self.sync()?,
            FsyncPolicy::EveryN(n) if self.unsynced >= n => self.sync()?,
            _ => {}
        }
        Ok(())
    }

    /// Force l'écriture sur disque.
    pub fn sync(&mut self) -> Result<(), StoreError> {
        self.file.sync_all()?;
        self.unsynced = 0;
        Ok(())
    }

    fn rotate(&mut self) -> Result<(), StoreError> {
        self.sync()?;
        self.number += 1;
        self.file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(path_of(&self.config.dir, self.number))?;
        self.size = 0;
        Ok(())
    }

    /// Numéro du segment courant.
    pub fn current_segment(&self) -> u64 {
        self.number
    }
}

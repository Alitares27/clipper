use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::io;
use std::path::Path;

const LIMIT: i64 = 25;
const PRUNE: &str = "DELETE FROM history WHERE pinned = 0 AND id IN (
    SELECT id FROM history WHERE pinned = 0 ORDER BY position DESC LIMIT -1 OFFSET ?1
)";

pub struct Storage(Connection);

impl Storage {
    pub fn open(path: &Path, password: &str) -> io::Result<Self> {
        if password.is_empty() || password.contains('\0') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "frase de paso invalida"));
        }

        let connection = Connection::open(path).map_err(io::Error::other)?;
        connection.pragma_update(None, "key", password).map_err(io::Error::other)?;
        let _: String = connection
            .query_row("PRAGMA cipher_version", [], |row| row.get(0))
            .map_err(|_| io::Error::other("SQLCipher no esta disponible"))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL UNIQUE,
                pinned INTEGER NOT NULL DEFAULT 0,
                position INTEGER NOT NULL
            );",
        ).map_err(|error| {
            if error.sqlite_error_code() == Some(rusqlite::ErrorCode::NotADatabase) {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "no se pudo abrir el historial. Comprueba la frase de paso y el archivo elegido",
                )
            } else {
                io::Error::other(error)
            }
        })?;
        Ok(Self(connection))
    }

    pub fn capture(&mut self, text: &str) -> rusqlite::Result<()> {
        let transaction = self.0.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO history (text, position)
             VALUES (?1, (SELECT coalesce(max(position), 0) + 1 FROM history))
             ON CONFLICT(text) DO UPDATE SET position = excluded.position",
            [text],
        )?;
        transaction.execute(PRUNE, [LIMIT])?;
        transaction.commit()
    }

    pub fn entries(&self) -> rusqlite::Result<Vec<(u64, String, bool)>> {
        let mut statement = self.0.prepare(
            "SELECT id, text, pinned FROM history ORDER BY position DESC",
        )?;
        let entries = statement.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        entries.collect()
    }

    pub fn get(&self, id: u64) -> rusqlite::Result<Option<String>> {
        self.0.query_row("SELECT text FROM history WHERE id = ?1", [id], |row| row.get(0)).optional()
    }

    pub fn remove(&self, id: u64) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM history WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn clear(&self) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn pin(&mut self, id: u64, pinned: bool) -> rusqlite::Result<bool> {
        let transaction = self.0.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE history SET pinned = ?1 WHERE id = ?2", params![pinned, id],
        )?;
        transaction.execute(PRUNE, [LIMIT])?;
        transaction.commit()?;
        Ok(changed > 0)
    }

    pub fn len(&self) -> rusqlite::Result<usize> {
        self.0.query_row("SELECT count(*) FROM history", [], |row| row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_history_survives_reopen_and_retains_pinned_entries() {
        let directory = std::env::temp_dir().join(format!("clipper-test-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("history.db");
        let key = "test passphrase with 'quote'";
        {
            let mut storage = Storage::open(&path, key).unwrap();
            storage.capture("entrada fijada").unwrap();
            assert!(storage.pin(1, true).unwrap());
            for index in 0..=LIMIT {
                storage.capture(&format!("entrada {index}")).unwrap();
            }
            storage.capture("entrada 1").unwrap();
        }

        assert!(!std::fs::read(&path).unwrap().starts_with(b"SQLite format 3"));
        assert!(Storage::open(&path, "incorrecta").is_err());
        let mut storage = Storage::open(&path, key).unwrap();
        assert_eq!(storage.len().unwrap(), LIMIT as usize + 1);
        assert_eq!(storage.get(1).unwrap().as_deref(), Some("entrada fijada"));
        assert_eq!(storage.get(2).unwrap(), None);
        assert_eq!(storage.entries().unwrap()[0].0, 3);
        storage.pin(1, false).unwrap();
        assert_eq!(storage.get(1).unwrap(), None);
        storage.remove(3).unwrap();
        assert_eq!(storage.get(3).unwrap(), None);
        storage.clear().unwrap();
        assert_eq!(storage.len().unwrap(), 0);
        drop(storage);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

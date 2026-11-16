//! Where the database key lives. The desktop shell implements `KeyStore` over the OS keychain;
//! tests use `MemoryKeyStore`. The key never touches the database folder.
use std::cell::RefCell;

pub trait KeyStore {
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, key: &str) -> Result<(), String>;
}

#[derive(Default)]
pub struct MemoryKeyStore(RefCell<Option<String>>);

impl KeyStore for MemoryKeyStore {
    fn get(&self) -> Result<Option<String>, String> {
        Ok(self.0.borrow().clone())
    }
    fn set(&self, key: &str) -> Result<(), String> {
        *self.0.borrow_mut() = Some(key.to_string());
        Ok(())
    }
}

/// A fresh 256-bit key as hex.
pub fn generate_key() -> Result<String, String> {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// The stored key, creating and saving one on first run.
pub fn key_or_create(store: &dyn KeyStore) -> Result<(String, bool), String> {
    if let Some(k) = store.get()? {
        return Ok((k, false));
    }
    let k = generate_key()?;
    store.set(&k)?;
    Ok((k, true))
}

#[cfg(feature = "sqlcipher")]
pub use private::open_private;

#[cfg(feature = "sqlcipher")]
mod private {
    use super::*;
    use fin_db::Db;
    use std::path::Path;

    /// Open the household database encrypted with the keychain key. A database left by an
    /// older unencrypted build is upgraded in place first. If the file exists but the key is
    /// missing (and the file is already encrypted) this fails rather than starting empty.
    pub fn open_private(path: &Path, store: &dyn KeyStore) -> Result<Db, String> {
        let existing = path.exists();
        let (key, created) = key_or_create(store)?;
        if existing {
            if Db::is_plain_file(path) {
                Db::encrypt_plain_file(path, &key).map_err(|e| e.to_string())?;
            } else if created {
                return Err("The encryption key for your data is missing from the system keychain, so the existing database cannot be opened. Restore the key or move the file aside to start fresh.".into());
            }
        }
        Db::open_encrypted(path, &key).map_err(|e| e.to_string())
    }
}

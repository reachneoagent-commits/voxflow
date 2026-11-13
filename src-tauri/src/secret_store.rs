//! API keys in the operating system's credential store (the macOS keychain, Windows Credential
//! Manager, the desktop's Secret Service on Linux) instead of the plain-text settings file.
//!
//! The settings code keeps working with one map of provider id to key. [`SecretStore::seal`]
//! turns that map into what is written to the file: a key the system's store accepted is blanked
//! there; a key it did not accept stays in the file, so a missing or locked store (a Linux
//! desktop without a Secret Service, say) never loses a key. [`SecretStore::hydrate`] fills the
//! blanks back in on load. A key found in the file from an older version is therefore moved into
//! the store the first time settings are read.
//!
//! Portable mode keeps everything in its own folder, so it never uses the system's store.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use keyring_core::{CredentialStore, Entry, Error};
use log::warn;

/// What the entries are filed under in the system's store. Together with the entry's name it is
/// how a key is found again, so it never changes.
const SERVICE: &str = "com.voxflow.app";

type Connect = Box<dyn Fn() -> keyring_core::Result<Arc<CredentialStore>> + Send + Sync>;

pub struct SecretStore {
    connect: Connect,
    /// The store once opened; opened again after it could not be reached.
    store: Mutex<Option<Arc<CredentialStore>>>,
    /// What the system's store is known to hold, by provider id (empty: nothing). Reading the
    /// store costs a round trip, and a keychain may ask the owner to unlock it, so each key is
    /// read once.
    known: Mutex<HashMap<String, String>>,
    warned: AtomicBool,
}

impl SecretStore {
    fn with(connect: Connect) -> Self {
        Self {
            connect,
            store: Mutex::new(None),
            known: Mutex::new(HashMap::new()),
            warned: AtomicBool::new(false),
        }
    }

    /// The system's own store, or none (keys stay in the file) in portable mode.
    fn system() -> Self {
        if crate::portable::is_portable() {
            return Self::with(Box::new(|| Err(Error::NoDefaultStore)));
        }
        Self::with(Box::new(connect))
    }

    fn store(&self) -> keyring_core::Result<Arc<CredentialStore>> {
        let mut kept = self.store.lock().map_err(|_| Error::NoDefaultStore)?;
        if let Some(store) = kept.as_ref() {
            return Ok(store.clone());
        }
        let store = (self.connect)()?;
        *kept = Some(store.clone());
        Ok(store)
    }

    fn entry(&self, provider: &str) -> keyring_core::Result<Entry> {
        self.store()?
            .build(SERVICE, &format!("post_process_api_key:{provider}"), None)
    }

    /// Note that the store could not be used. Said once, and never with the error's own words:
    /// a platform error may echo the secret that was being stored.
    fn unavailable(&self) {
        if !self.warned.swap(true, Ordering::Relaxed) {
            warn!("The system's credential store is not available; API keys stay in the settings file");
        }
        // Open it again next time rather than keep a connection that is gone.
        if let Ok(mut store) = self.store.lock() {
            *store = None;
        }
    }

    fn remember(&self, provider: &str, value: &str) {
        if let Ok(mut known) = self.known.lock() {
            known.insert(provider.to_string(), value.to_string());
        }
    }

    fn recall(&self, provider: &str) -> Option<String> {
        self.known.lock().ok()?.get(provider).cloned()
    }

    /// The key kept for `provider`, if the system's store has one.
    fn read(&self, provider: &str) -> Option<String> {
        if let Some(value) = self.recall(provider) {
            return Some(value).filter(|v| !v.is_empty());
        }
        match self.entry(provider).and_then(|entry| entry.get_password()) {
            Ok(value) => {
                self.remember(provider, &value);
                Some(value).filter(|v| !v.is_empty())
            }
            Err(Error::NoEntry) => {
                self.remember(provider, "");
                None
            }
            Err(_) => {
                self.unavailable();
                None
            }
        }
    }

    /// Make the system's store hold `secret` for `provider` (empty: nothing). `false` when it
    /// could not, in which case the caller keeps the key itself.
    fn write(&self, provider: &str, secret: &str) -> bool {
        if self.recall(provider).as_deref() == Some(secret) {
            return true;
        }
        let done = self.entry(provider).and_then(|entry| {
            if secret.is_empty() {
                match entry.delete_credential() {
                    Ok(()) | Err(Error::NoEntry) => Ok(()),
                    Err(e) => Err(e),
                }
            } else {
                entry.set_password(secret)
            }
        });
        match done {
            Ok(()) => {
                self.remember(provider, secret);
                true
            }
            Err(_) => {
                self.unavailable();
                false
            }
        }
    }

    /// Fill in, from the system's store, each key in `keys` (for `providers`) that is blank.
    /// A key already in the map wins: it is newer than the store's (an older version wrote it).
    pub fn hydrate<'a>(
        &self,
        keys: &mut HashMap<String, String>,
        providers: impl IntoIterator<Item = &'a str>,
    ) {
        for provider in providers {
            if keys.get(provider).is_some_and(|key| !key.is_empty()) {
                continue;
            }
            if let Some(key) = self.read(provider) {
                keys.insert(provider.to_string(), key);
            }
        }
    }

    /// What to write to the settings file for `keys`: the same map, with each key the system's
    /// store now holds blanked. A key cleared in `keys` is removed from the store.
    pub fn seal(&self, keys: &HashMap<String, String>) -> HashMap<String, String> {
        let mut sealed = keys.clone();
        for (provider, key) in keys {
            if self.write(provider, key) {
                sealed.insert(provider.clone(), String::new());
            }
        }
        sealed
    }
}

/// The system's own store.
fn connect() -> keyring_core::Result<Arc<CredentialStore>> {
    #[cfg(target_os = "windows")]
    let store: Arc<CredentialStore> = windows_native_keyring_store::Store::new()?;
    #[cfg(target_os = "macos")]
    let store: Arc<CredentialStore> = apple_native_keyring_store::keychain::Store::new()?;
    #[cfg(target_os = "linux")]
    let store: Arc<CredentialStore> = zbus_secret_service_keyring_store::Store::new()?;
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let store: Arc<CredentialStore> = return Err(Error::NoDefaultStore);
    #[allow(unreachable_code)]
    Ok(store)
}

/// The process-wide store.
pub fn secrets() -> &'static SecretStore {
    static STORE: OnceLock<SecretStore> = OnceLock::new();
    STORE.get_or_init(SecretStore::system)
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring_core::mock;

    fn mocked() -> (SecretStore, Arc<CredentialStore>) {
        let mock: Arc<CredentialStore> = mock::Store::new().unwrap();
        let handed = mock.clone();
        (
            SecretStore::with(Box::new(move || Ok(handed.clone()))),
            mock,
        )
    }

    fn broken() -> SecretStore {
        SecretStore::with(Box::new(|| Err(Error::NoDefaultStore)))
    }

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_stored_key_is_blanked_in_the_file_and_comes_back_on_load() {
        let (store, _) = mocked();
        let sealed = store.seal(&map(&[("openai", "sk-secret"), ("groq", "")]));
        assert_eq!(sealed, map(&[("openai", ""), ("groq", "")]));

        // A fresh process: nothing cached, only the system's store.
        let mut loaded = sealed;
        store.hydrate(&mut loaded, ["openai", "groq"]);
        assert_eq!(loaded["openai"], "sk-secret");
        assert_eq!(loaded["groq"], "");
    }

    #[test]
    fn the_key_really_is_in_the_system_store_under_a_stable_name() {
        let (store, mock) = mocked();
        store.seal(&map(&[("openai", "sk-secret")]));
        let entry = mock
            .build("com.voxflow.app", "post_process_api_key:openai", None)
            .unwrap();
        assert_eq!(entry.get_password().unwrap(), "sk-secret");
    }

    #[test]
    fn a_key_from_an_older_version_in_the_file_is_moved_not_lost() {
        let (store, mock) = mocked();
        let mut loaded = map(&[("openai", "sk-old")]);
        store.hydrate(&mut loaded, ["openai"]);
        assert_eq!(loaded["openai"], "sk-old");
        assert_eq!(store.seal(&loaded)["openai"], "");
        let entry = mock
            .build("com.voxflow.app", "post_process_api_key:openai", None)
            .unwrap();
        assert_eq!(entry.get_password().unwrap(), "sk-old");
    }

    #[test]
    fn clearing_a_key_removes_it_from_the_system_store() {
        let (store, mock) = mocked();
        store.seal(&map(&[("openai", "sk-secret")]));
        let sealed = store.seal(&map(&[("openai", "")]));
        assert_eq!(sealed["openai"], "");
        let entry = mock
            .build("com.voxflow.app", "post_process_api_key:openai", None)
            .unwrap();
        assert!(matches!(entry.get_password(), Err(Error::NoEntry)));
        let mut loaded = sealed;
        store.hydrate(&mut loaded, ["openai"]);
        assert_eq!(loaded["openai"], "");
    }

    #[test]
    fn replacing_a_key_replaces_it_in_the_system_store() {
        let (store, _) = mocked();
        store.seal(&map(&[("openai", "one")]));
        store.seal(&map(&[("openai", "two")]));
        let (fresh, _) = (
            SecretStore::with(Box::new({
                let s = store.store().unwrap();
                move || Ok(s.clone())
            })),
            (),
        );
        let mut loaded = map(&[("openai", "")]);
        fresh.hydrate(&mut loaded, ["openai"]);
        assert_eq!(loaded["openai"], "two");
    }

    #[test]
    fn without_a_system_store_the_key_stays_in_the_file() {
        let store = broken();
        let sealed = store.seal(&map(&[("openai", "sk-secret"), ("groq", "")]));
        assert_eq!(sealed["openai"], "sk-secret");
        let mut loaded = sealed;
        store.hydrate(&mut loaded, ["openai", "groq"]);
        assert_eq!(loaded["openai"], "sk-secret");
    }

    #[test]
    fn a_key_in_the_file_wins_over_an_older_one_in_the_store() {
        let (store, _) = mocked();
        store.seal(&map(&[("openai", "stored")]));
        let mut loaded = map(&[("openai", "typed in an older version")]);
        store.hydrate(&mut loaded, ["openai"]);
        assert_eq!(loaded["openai"], "typed in an older version");
        assert_eq!(store.seal(&loaded)["openai"], "");
    }
}

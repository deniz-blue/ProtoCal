use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore as _;

const GRANT_LIFETIME: Duration = Duration::from_secs(60 * 60);

pub fn random_token() -> String {
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub struct GrantStore<T> {
    grants: Mutex<HashMap<String, (T, Instant)>>,
}

impl<T> Default for GrantStore<T> {
    fn default() -> Self {
        Self { grants: Mutex::default() }
    }
}

impl<T: Clone> GrantStore<T> {
    pub fn grant(&self, payload: T) -> String {
        let token = random_token();
        self.lock().insert(token.clone(), (payload, Instant::now() + GRANT_LIFETIME));
        token
    }

    pub fn find(&self, token: &str) -> Option<T> {
        let mut grants = self.lock();
        let (payload, expires_at) = grants.get(token)?;
        if *expires_at > Instant::now() {
            return Some(payload.clone());
        }
        grants.remove(token);
        None
    }

    pub fn spend(&self, token: &str) {
        self.lock().remove(token);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, (T, Instant)>> {
        self.grants.lock().expect("grant store lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grant_is_found_until_it_is_spent() {
        let store = GrantStore::default();
        let token = store.grant(7);
        assert_eq!(URL_SAFE_NO_PAD.decode(&token).unwrap().len(), 24);
        assert_eq!(store.find(&token), Some(7));
        store.spend(&token);
        assert_eq!(store.find(&token), None);
    }
}

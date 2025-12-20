use std::fmt::Display;
use std::ops::Deref;

use chrono::{DateTime, Utc};
use chrono_tz::America::Los_Angeles;
use chrono_tz::Tz;

pub struct Key {
    pub key: String,
    pub ratelimited_at: Option<DateTime<Tz>>,
}
impl From<String> for Key {
    fn from(s: String) -> Self {
        Self {
            key: s,
            ratelimited_at: None,
        }
    }
}
impl Deref for Key {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}
impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key)
    }
}

pub struct KeyJuggler {
    keys: Vec<Key>,
    current_index: usize,
}

impl KeyJuggler {
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            keys: keys.into_iter().map(Key::from).collect(),
            current_index: 0,
        }
    }

    fn next(&mut self) -> usize {
        self.current_index += 1;
        if self.current_index >= self.keys.len() {
            self.current_index = 0;
        }

        self.current_index
    }

    fn next_unratelimited(&mut self) -> Option<&Key> {
        let old_index = self.current_index;
        loop {
            let next_index = self.next();

            if next_index == old_index {
                log::warn!("All keys are ratelimited, returning None");
                return None;
            }

            if !Self::is_ratelimited(&mut self.keys[next_index]) {
                log::debug!(
                    "Key {} is not ratelimited, returning it",
                    self.keys[next_index].key
                );
                return Some(&self.keys[next_index]);
            }

            log::debug!(
                "Key {} is ratelimited, trying next key",
                self.keys[next_index].key
            );
        }
    }

    /// ratelimits the current key and returns the next unratelimited key
    pub fn ratelimit(&mut self) -> Option<&Key> {
        log::warn!("Ratelimiting key {}", self.keys[self.current_index].key);

        let current_time = Utc::now().with_timezone(&Los_Angeles);

        let current_key = &mut self.keys[self.current_index];
        current_key.ratelimited_at = Some(current_time);

        self.next_unratelimited()
    }

    /// returns the current key
    pub fn current(&self) -> &Key {
        let key = &self.keys[self.current_index];

        log::debug!("Current key is {}", key.key);

        key
    }

    fn is_ratelimited(key: &mut Key) -> bool {
        let current_time = Utc::now().with_timezone(&Los_Angeles);

        if let Some(ratelimited_at) = key.ratelimited_at {
            if (current_time - ratelimited_at) > chrono::Duration::days(1) {
                // key is no longer ratelimited
                key.ratelimited_at = None;
                false
            } else {
                // key is still ratelimited
                true
            }
        } else {
            // key was never ratelimited
            false
        }
    }
}

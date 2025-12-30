use std::fmt::Display;
use std::ops::Deref;

use chrono::{DateTime, Utc};
use colored::Colorize;
use log::{debug, info};
use rand::seq::SliceRandom;
use serde::Serialize;

pub struct Key {
    pub key: String,
    pub ratelimited_at: Option<DateTime<Utc>>,
    pub num_requests: u64,
}

impl From<String> for Key {
    fn from(key: String) -> Self {
        Self {
            key,
            ratelimited_at: None,
            num_requests: 0,
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

#[derive(Serialize)]
pub struct KeyStatus {
    pub index: usize,
    pub key_masked: String,
    pub num_requests: u64,
    pub is_ratelimited: bool,
    pub seconds_remaining: Option<i64>,
}

pub struct KeyJuggler {
    keys: Vec<Key>,
}

impl KeyJuggler {
    pub fn new(keys: Vec<String>) -> Self {
        info!(
            "initializing key juggler with {} {}",
            keys.len().to_string().cyan().bold(),
            if keys.len() == 1 { "key" } else { "keys" }
        );
        let mut keys: Vec<Key> = keys.into_iter().map(Key::from).collect();
        keys.shuffle(&mut rand::rng());
        Self { keys }
    }

    pub fn select(&mut self) -> Option<&Key> {
        let best_idx = self.find_best_key()?;
        self.keys[best_idx].num_requests += 1;
        debug!(
            "selected key {} (index {}, {} total {})",
            self.keys[best_idx].key.cyan(),
            best_idx.to_string().cyan(),
            self.keys[best_idx].num_requests.to_string().cyan(),
            if self.keys[best_idx].num_requests == 1 {
                "request"
            } else {
                "requests"
            }
        );
        Some(&self.keys[best_idx])
    }

    fn find_best_key(&mut self) -> Option<usize> {
        let current_time = Utc::now();

        let mut best_idx: Option<usize> = None;
        let mut best_score: Option<i64> = None;

        for (idx, key) in self.keys.iter_mut().enumerate() {
            let is_expired = match key.ratelimited_at {
                None => false,
                Some(ratelimited_at) => {
                    if (current_time - ratelimited_at) > chrono::Duration::days(1) {
                        key.ratelimited_at = None;
                        false
                    } else {
                        true
                    }
                }
            };

            let score: i64 = match is_expired {
                true => i64::MAX / 2,
                false => key.num_requests as i64,
            };

            match best_score {
                None => {
                    best_idx = Some(idx);
                    best_score = Some(score);
                }
                Some(current_best) if score < current_best => {
                    best_idx = Some(idx);
                    best_score = Some(score);
                }
                _ => {}
            }
        }

        best_idx
    }

    pub fn ratelimit(&mut self, key: &str) -> Option<&Key> {
        if let Some(idx) = self.keys.iter().position(|k| k.key == key) {
            let request_count = self.keys[idx].num_requests;
            log::warn!(
                "ratelimited key {} at index {} (handled {} {})",
                self.keys[idx].key.cyan(),
                idx.to_string().cyan(),
                request_count.to_string().cyan(),
                if request_count == 1 {
                    "request"
                } else {
                    "requests"
                }
            );
            self.keys[idx].ratelimited_at = Some(Utc::now());
            self.keys[idx].num_requests = 0;
            self.select()
        } else {
            log::warn!("key not found for ratelimit");
            None
        }
    }

    pub fn remove(&mut self, key: &str) {
        if let Some(idx) = self.keys.iter().position(|k| k.key == key) {
            log::warn!(
                "removing key {} at index {} from rotation",
                self.keys[idx].key.cyan(),
                idx.to_string().cyan()
            );
            self.keys.remove(idx);
        }
    }

    pub fn current(&mut self) -> &Key {
        let idx = self.find_best_key().unwrap_or(0);
        self.keys[idx].num_requests += 1;
        &self.keys[idx]
    }

    pub fn get_status(&mut self) -> Vec<KeyStatus> {
        let current_time = Utc::now();

        self.keys
            .iter_mut()
            .enumerate()
            .map(|(idx, key)| {
                let is_expired = match key.ratelimited_at {
                    None => false,
                    Some(ratelimited_at) => {
                        if (current_time - ratelimited_at) > chrono::Duration::days(1) {
                            key.ratelimited_at = None;
                            false
                        } else {
                            true
                        }
                    }
                };

                let seconds_remaining = if is_expired {
                    let remaining =
                        chrono::Duration::days(1) - (current_time - key.ratelimited_at.unwrap());
                    Some(remaining.num_seconds())
                } else {
                    None
                };

                KeyStatus {
                    index: idx,
                    key_masked: format!("{}...{}", &key.key[..6], &key.key[key.key.len() - 4..]),
                    num_requests: key.num_requests,
                    is_ratelimited: is_expired,
                    seconds_remaining,
                }
            })
            .collect()
    }
}

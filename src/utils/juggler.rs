use std::fmt::Display;
use std::ops::Deref;

use chrono::{DateTime, Utc};
use chrono_tz::America::Los_Angeles;
use chrono_tz::Tz;
use colored::Colorize;

pub struct Key {
    pub key: String,
    pub ratelimited_at: Option<DateTime<Tz>>,
}

impl From<String> for Key {
    fn from(key: String) -> Self {
        Self { key, ratelimited_at: None }
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
}

impl KeyJuggler {
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            keys: keys.into_iter().map(Key::from).collect(),
        }
    }

    pub fn select(&mut self) -> Option<&Key> {
        let best_idx = self.find_best_key()?;
        log::debug!("selected key {} (index {})", self.keys[best_idx].key.cyan(), best_idx.to_string().cyan());
        Some(&self.keys[best_idx])
    }

    fn find_best_key(&mut self) -> Option<usize> {
        let current_time = Utc::now().with_timezone(&Los_Angeles);

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

            let score = match is_expired {
                true => {
                    let remaining = chrono::Duration::days(1) - (current_time - key.ratelimited_at.unwrap());
                    remaining.num_seconds()
                }
                false => -1,
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

    pub fn ratelimit(&mut self) -> Option<&Key> {
        if let Some(idx) = self.find_best_key() {
            log::warn!("ratelimited key {} at index {}", self.keys[idx].key.cyan(), idx.to_string().cyan());
            self.keys[idx].ratelimited_at = Some(Utc::now().with_timezone(&Los_Angeles));
            self.select()
        } else {
            log::warn!("all {} are ratelimited", "keys".bright_red().bold());
            None
        }
    }

    pub fn current(&mut self) -> &Key {
        let idx = self.find_best_key().unwrap_or(0);
        &self.keys[idx]
    }
}

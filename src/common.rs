//Author Josiah Bull, Copyright 2021
//! A collection of small useful helper functions.
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_time_seconds() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as i64
}
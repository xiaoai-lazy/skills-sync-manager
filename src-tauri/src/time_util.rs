use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> String {
    timestamp_nanos().to_string()
}

pub fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos()
}

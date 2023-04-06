
use astrolabe::DateTime;
use astrolabe::Precision;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use sha1::{Digest, Sha1};


pub fn format_duration(duration_ms: u64) -> String {
    let duration = std::time::Duration::from_millis(duration_ms);
    let seconds = duration.as_secs();

    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    let milliseconds = duration.subsec_micros();

    format!("{:02}.{:02}:{:02}:{:02}.{:06}", days, hours, minutes, seconds, milliseconds)
}

pub fn to_iso8601_utc(now: SystemTime) -> String {
    let timestamp =  now.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let date_time = DateTime::from_timestamp(timestamp.try_into().unwrap()).unwrap();
    
    date_time.format_rfc3339(Precision::Millis)
}

pub fn uuid(now: SystemTime) -> String {
    let text = now.duration_since(UNIX_EPOCH).unwrap().as_nanos().to_string();
    let mut hasher = Sha1::new();
    hasher.update(text);
    let result = hasher.finalize();
    let hash_hex = format!("{:x}", result);
    let (part1, remainder) = hash_hex.split_at(16);
    let (part2, remainder) = remainder.split_at(4);
    let (part3, remainder) = remainder.split_at(4);
    let (part4, part5) = remainder.split_at(4);
    
    format!("sha1 hash: {}-{}-{}-{}-{}", part1, part2, part3, part4, part5)       
}

#[test]
fn test_now() {
    println!("now: {}", to_iso8601_utc(SystemTime::now()));
}


/// Current unix time in seconds.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("congratulations, you travelled back to before 1970, now fix your clock")
        .as_secs() as i64
}

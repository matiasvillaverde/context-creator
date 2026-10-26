pub const default_ttl: u64 = 3600;

pub fn elapsed(issued_at: u64, now: u64) u64 {
    return now -| issued_at;
}

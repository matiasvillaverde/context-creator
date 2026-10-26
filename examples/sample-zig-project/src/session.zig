const policy = @import("policy.zig");

pub const Session = struct {
    issued_at: u64,
    ttl_seconds: u64 = policy.default_ttl,

    pub fn expired(self: @This(), now: u64) bool {
        return policy.elapsed(self.issued_at, now) >= self.ttl_seconds;
    }
};

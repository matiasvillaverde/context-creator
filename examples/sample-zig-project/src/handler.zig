const std = @import("std");
const session = @import("session.zig");

pub fn authorize(value: session.Session, now: u64) bool {
    return !value.expired(now);
}

test "authorization expires at the session deadline" {
    try std.testing.expect(authorize(.{ .issued_at = 100 }, 120));
    try std.testing.expect(!authorize(.{ .issued_at = 100 }, 3700));
}

const handler = @import("handler.zig");

pub fn main() void {
    _ = handler.authorize(.{ .issued_at = 100 }, 120);
}

# Zig dependency context example

From the repository root:

```bash
cargo run -- context --root examples/sample-zig-project \
  --seed src/main.zig --max-tokens 12000
```

The local import graph follows `main.zig` → `handler.zig` → `session.zig` →
`policy.zig`, preserving the `Session` fields, method, and authorization test.
The `std` implementation remains an external gap. The build script is returned
as configuration and is never executed by context-creator.

With Zig installed, run the example's actual tests separately:

```bash
cd examples/sample-zig-project
zig build test
```

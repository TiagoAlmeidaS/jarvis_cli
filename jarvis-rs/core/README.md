# jarvis-core

This crate implements the business logic for jarvis. It is designed to be used by the various jarvis UIs written in Rust.

## Dependencies

Note that `jarvis-core` makes some assumptions about certain helper utilities being available in the environment. Currently, this support matrix is:

### macOS

Expects `/usr/bin/sandbox-exec` to be present.

When using the workspace-write sandbox policy, the Seatbelt profile allows
writes under the configured writable roots while keeping `.git` (directory or
pointer file), the resolved `gitdir:` target, and `.jarvis` read-only.

### Linux

Expects the binary containing `jarvis-core` to run the equivalent of `jarvis sandbox linux` (legacy alias: `jarvis debug landlock`) when `arg0` is `jarvis-linux-sandbox`. See the `jarvis-arg0` crate for details.

### All Platforms

Expects the binary containing `jarvis-core` to simulate the virtual `apply_patch` CLI when `arg1` is `--jarvis-run-as-apply-patch`. See the `jarvis-arg0` crate for details.

# OTRSP Issue Remediation Plan

This plan addresses the three review findings:
1. Startup `?NAME` timeout can leave stale bytes that corrupt the next query.
2. `query_aux()` does not validate that returned AUX port matches requested port.
3. `Disconnected` event emission is inconsistent across shutdown/error paths.

## Goals

- Keep API surface stable unless a correctness fix requires additive API.
- Preserve existing behavior for successful command paths.
- Make failure behavior deterministic and testable.
- Add regression tests that fail before fixes and pass after fixes.

## Scope

- In-scope files:
  - `src/builder.rs`
  - `src/device.rs`
  - `src/io.rs`
  - `src/transport.rs` (only if mock behavior needs extension for deterministic tests)
  - `tests/integration.rs`
  - potentially new targeted test file(s) under `tests/`
- Out-of-scope:
  - protocol command set changes
  - public trait redesign
  - non-OTRSP transport backends

## Implementation Strategy

Implement in three phases, each with its own tests. Land phases in order so each change can be verified independently.

## Phase 1: Prevent stale startup response poisoning

### Problem
`OtrspBuilder::build_with_port()` sends `?NAME` and times out after 1 second. On timeout, it proceeds without draining a late response. That late line may be consumed by the next query (`device_name`, `query_aux`) and be misinterpreted.

### Design
Use a single-reader model for line responses once the IO task starts, and avoid pre-consuming serial data in a way that can leave undrained bytes.

Preferred minimal-risk approach:
- Keep optional name query in builder, but if timeout or read error occurs, perform a bounded drain pass before spawning the IO task.
- Drain policy:
  - Short window (for example 100–250 ms total)
  - Read until no bytes arrive within a short sub-timeout (for example 10–20 ms)
  - Stop on EOF/error
- Never block startup indefinitely.

Alternative (if complexity is lower after coding):
- Remove pre-spawn `?NAME` read logic entirely.
- Spawn IO task first and perform name query through `io.command_read()` during build.
- This fully centralizes read ownership and naturally prevents stale cross-consumption.
- If adopted, ensure startup still returns `Unknown` instead of failing hard on name timeout.

### Code Changes
- `src/builder.rs`
  - Refactor name-query block to ensure no unread late line remains after timeout/error.
  - Add helper for bounded drain if keeping current pre-spawn query model.
  - Keep `query_name(false)` behavior unchanged.
- `src/io.rs` (only if alternative approach is used)
  - If builder moves to IO-owned query, ensure helper method is available in builder flow without exposing internals publicly.

### Tests
Add/adjust integration tests to reproduce stale-buffer scenario:
- `build_name_timeout_does_not_corrupt_next_query`
  - Build with `query_name(true)`.
  - Simulate no immediate response so builder times out.
  - After build completes, enqueue expected AUX response and verify `query_aux()` parses correctly.
  - Also simulate a late name response and ensure it does not satisfy next AUX query.

Mock/testability notes:
- If current `MockPort` cannot model delayed enqueue timing clearly, add minimal support utilities in `src/transport.rs` (for example async delayed `queue_read`) used only by tests.

### Acceptance Criteria
- After a builder name timeout, first subsequent `command_read` still consumes its own response.
- No regressions in existing integration tests.

## Phase 2: Validate AUX response port integrity

### Problem
`query_aux(port)` parses `(returned_port, value)` but ignores `returned_port`.

### Design
Treat response port mismatch as protocol error.

### Code Changes
- `src/device.rs`
  - In `query_aux`, compare requested `port` with parsed `returned_port`.
  - On mismatch, return `Error::Protocol` with a precise message including both ports.

### Tests
- `query_aux_rejects_mismatched_port`
  - Request `?AUX1`.
  - Queue `AUX24\r`.
  - Assert error variant is `Error::Protocol`.
- Retain existing success-path test (`query_aux_via_trait`) as control.

### Acceptance Criteria
- Mismatched AUX response no longer returns success.
- Error message is actionable for diagnosing stream desync or device anomalies.

## Phase 3: Normalize Disconnected event semantics

### Problem
`Disconnected` is not emitted in all disconnect scenarios:
- graceful shutdown path returns before tail emission
- read-side I/O errors in `WriteAndRead` return error without emitting disconnect

### Design
Define explicit event semantics:
- Emit `Disconnected` exactly once when IO task exits or detects unrecoverable transport failure.
- Do not emit duplicate disconnect events for a single failure sequence.

### Code Changes
- `src/io.rs`
  - Ensure shutdown path goes through common exit logic that emits `Disconnected`.
  - Emit disconnect on read-side I/O errors in `WriteAndRead`.
  - Add local guard (boolean) to avoid duplicate emission from multiple error branches.
  - Keep timeout behavior (`Error::Timeout`) non-fatal unless policy decision says otherwise.

### Tests
- `close_emits_disconnected_event`
  - Subscribe before `close()` and assert `Disconnected` arrives.
- `read_error_emits_disconnected_event`
  - Force read error (for example close mock during pending read) and assert event emitted.
- `single_disconnected_event_on_failure`
  - Trigger write/read error path and ensure receiver sees one disconnect event for one teardown sequence.

### Acceptance Criteria
- Event consumers can reliably infer connectivity loss on shutdown and I/O failure.
- No event floods or duplicated `Disconnected` emissions for one disconnect.

## Cross-Cutting Work

### Error taxonomy review
- Confirm chosen variants are consistent:
  - protocol mismatch => `Error::Protocol`
  - transport EOF/broken pipe => `Error::Io` and disconnect event
  - command wait timeout => `Error::Timeout`

### Documentation updates
- `README.md`
  - Add one short note describing disconnect event behavior.
  - Optionally mention startup name query is best-effort and does not compromise subsequent queries.

### Observability
- Keep or add debug logs around:
  - startup name timeout + drain activity
  - AUX response mismatch
  - disconnect emission reason

## Execution Order

1. Phase 2 first (smallest, lowest risk, quick correctness gain).
2. Phase 3 second (event behavior and lifecycle consistency).
3. Phase 1 last (most subtle stream-ordering behavior; easier after event semantics are stable).
4. Run full tests and clippy after each phase.

## Validation Checklist

After implementation:
- `cargo test`
- `cargo clippy --all-targets --all-features`
- Optional stress loop for flaky timing tests:
  - `for i in {1..50}; do cargo test --test integration -- --nocapture || break; done`

## Rollback Strategy

If Phase 1 introduces unstable timing behavior in tests:
- Temporarily switch to the alternative architecture (query via IO task post-spawn), which removes split read ownership.
- Keep Phase 2 and 3 changes intact; they are independent.

## Deliverables

- Code fixes in the listed source files.
- New/updated tests that reproduce and prevent regressions.
- Updated docs for event/disconnect semantics if behavior is user-visible.

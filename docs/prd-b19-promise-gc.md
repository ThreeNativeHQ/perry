# PRD-B19 — Promise microtask GC correctness

**Status:** DONE
**Priority:** P0 — concurrent async loads crash in the moving collector.
**Complexity:** 3 (LOW); risk override: none
**Owner:** Codex, fix/b19-promise-gc
**Depends on:** None

## Context
The supplied 25-load parse/decode reproducer crashes while the microtask pump
reads a promise's next link. Work stays in the existing B19 checkout at
9f50fd833a109d450bb4ae1c7b47ffa5afcc7325; no push or other checkout edits.

## Solution
Identify the missed or stale promise reference, reuse mutable GC roots and
reload moved pointers. Preserve collection and Node behavior. The consumer
path is compiled async loads → promise task queue → microtask pump → settlement.

## Acceptance Criteria
- [x] AC-1 [local]: reproducer regression fails before the fix (silent exit 0, no output) and matches Node after (8/8 runs) — proof: `scripts/prd581-promise-gc.sh`.
- [x] AC-2 [local]: focused runtime collection regression fails before and passes after (promise_executor test) — proof: focused `cargo test -p perry-runtime`.
- [x] AC-3 [local]: existing promise, async and GC tests pass, excluding the supplied typed_feedback baseline — proof: filtered runtime tests and native fixtures; exact commands/counts in `/tmp/b19-report.md`.
- [x] AC-4 [local]: release compiler and runtime/stdlib archives built, hashed, and one local commit recorded — proof: `cargo build --release -p perry -p perry-runtime-static -p perry-stdlib-static`, `sha256sum`, `git commit`.

## Integration Ledger
Integration: unchanged — existing compiled promise pump consumes corrected roots.

## Execution Phases
### Phase 1: Reproduce the collection failure
**Status:** DONE
**Files:** test-files/test_prd581_promise_gc.ts, scripts/prd581-promise-gc.sh
**Implementation:** preserve the supplied allocation-heavy parse/decode chain.
**Verification:** AC-1 red, locate the invalid root with targeted runtime reads.

### Phase 2: Correct the shared runtime reference
**Status:** DONE
**Files:** owning promise/GC module and its existing runtime tests.
**Implementation:** trace/update the actual missed reference; no GC kill switch.
**Verification:** AC-1/AC-2 green; inspect the diff against all callers.

### Phase 3: Validate and deliver
**Status:** DONE
**Implementation:** run affected suites, build releases under nice -n 19/-j 6,
commit once, and write the requested final report under 50 lines.
**Verification:** AC-3/AC-4. Retain this unmerged checkout and required artifacts.

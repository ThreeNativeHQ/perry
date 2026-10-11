//! B19: `new Promise(executor)` must return the post-collection promise.
//!
//! `js_promise_new_with_executor` ran user code (the executor) while the new
//! promise and the reject closure sat in bare Rust locals. A copying minor in
//! the executor moved the promise, so the constructor returned the pre-move
//! address and the caller later read a retired from-space promise (the
//! `js_async_step_chain` / microtask-pump crash on many concurrent loads).

use super::super::super::*;
use super::super::support::*;

extern "C" fn collect_then_resolve(
    _closure: *const crate::closure::ClosureHeader,
    resolve: f64,
    _reject: f64,
) -> f64 {
    // Collect BEFORE resolving, so only the relocated promise is fulfilled and a
    // stale pre-move copy would still read Pending.
    let scope = crate::gc::RuntimeHandleScope::new();
    let resolve_h = scope.root_nanbox_f64(resolve);
    crate::gc::gc_collect_minor();
    let resolve = (resolve_h.get_nanbox_f64().to_bits() & crate::value::POINTER_MASK)
        as *const crate::closure::ClosureHeader;
    crate::closure::js_closure_call1(resolve, 7.0);
    0.0
}

#[test]
fn promise_executor_result_survives_a_moving_minor_in_the_executor() {
    let _guard = CopyingNurseryTestGuard::new(0);
    let _scan = ConservativeScanDisabledGuard::new();
    let _triggers = GcTriggerThresholdTestGuard::suppress_automatic_triggers();
    let _force = ForcedEvacuationTestGuard::on();
    register_runtime_handle_root_scanner_for_tests();

    let executor = crate::closure::js_closure_alloc_singleton(collect_then_resolve as *const u8);
    let cycles_before = copying_minor_cycles();
    let promise = crate::promise::js_promise_new_with_executor(executor);
    assert!(
        copying_minor_cycles() > cycles_before,
        "the executor must run a copying minor"
    );
    assert!(
        crate::arena::pointer_in_nursery(promise as usize),
        "the returned promise must be a live to-space address"
    );
    unsafe {
        assert!(matches!((*promise).state, crate::promise::PromiseState::Fulfilled));
        assert_eq!((*promise).value, 7.0);
    }
}

/*
 * This will perform a synchronous fault in Dfinity (return non-zero on call_perform).
 * NOTE: This will instrument and fail EVERY call_perform as there is no predicate.
 *       to add granularity, another probe on call_new will need to be added to collect
 *       call site/target canister info.
 */
wasm:ic0:call_perform:alt {
    new_target_fn_name = inject_synchronous_fault;
}
## What I'd try next:

- Add #[inline] to the small callees above: TypedResource::rep, ResourceTables::resource_rep, ConcurrentState::get_mut, the TLS accessor, and the Result<(), ()> lowering.
- Make push_ overwrite a known-free slot without running Entry's drop.
- Avoid storing an empty BTreeSet in every Entry.
- Even so, some of the gap is layout and lost cross-function optimization that #[inline] can't fix. For comparing changes, keep benchmarking with codegen-units=1.

Calls main makes that the codegen1 build doesn't, from call-branch records (calls per request):

┌───────┬──────────────────────────────────────────────┬─────────────────────────────────┐
│ Calls │                    Callee                    │           Called from           │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 71    │ drop_glue::<resource_table::Entry>           │ ResourceTable::push_            │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ ~52   │ BTreeMap<u32, ()>::drop (each entry's empty  │ delete_maybe_debug::<…>         │
│       │ children set)                                │                                 │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 31    │ TypedResource::rep                           │ HandleTable::remove_resource    │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 15    │ ResourceTables::resource_rep                 │ transmit_origin,                │
│       │                                              │ host_reader_rep                 │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 13    │ delete_maybe_debug::<TransmitState>          │ delete_transmit                 │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 11.5  │ Lower for (Result<(),                        │ StaticHostFn result lowering    │
│       │ ()>,)::linear_lower_to_flat                  │                                 │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 11    │ COMPONENT_ASYNC_TLS accessor closure         │ poll_until, first_poll          │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 7     │ ConcurrentState::get_mut::<TransmitState>    │ guest_write, Source::read       │
├───────┼──────────────────────────────────────────────┼─────────────────────────────────┤
│ 5–7   │ catch_traps, invoke_wasm_and_catch_traps,    │ the wasm-call and host-call     │
│ each  │ call_raw, prepare_call, tls::try_get,        │ paths; one merged function in   │
│       │ StagedCall                                   │ the codegen1 build              │
└───────┴──────────────────────────────────────────────┴─────────────────────────────────┘

# Lightbeam

Lightbeam is an optimising one-pass streaming compiler for WebAssembly, intended for use in [Wasmtime][wasmtime].

[wasmtime]: https://github.com/CraneStation/wasmtime

## Specification compliance

It's hard to judge, since each test in the spec testsuite covers a wide range of features (to check their interactions), but currently 31 out of 77 of the spec suite tests pass when run in Wasmtime with Lightbeam as a backend. Here's the full test output:

```
running 76 tests
test misc_testsuite::stack_overflow         ... ok
test misc_testsuite::misc_traps             ... ok
test spec_testsuite::binary                 ... ok
test spec_testsuite::align                  ... FAILED
test spec_testsuite::block                  ... FAILED
test spec_testsuite::br_if                  ... FAILED
test spec_testsuite::break_drop             ... ok
test spec_testsuite::call                   ... FAILED
test spec_testsuite::call_indirect          ... FAILED
test spec_testsuite::comments               ... ok
test spec_testsuite::address                ... FAILED
test spec_testsuite::const_                 ... ok
test spec_testsuite::custom                 ... ok
test spec_testsuite::custom_section         ... ok
test spec_testsuite::data                   ... ok
test spec_testsuite::elem                   ... FAILED
test spec_testsuite::conversions            ... FAILED
test spec_testsuite::endianness             ... FAILED
test spec_testsuite::br                     ... ok
test spec_testsuite::exports                ... ok
test spec_testsuite::f32_bitwise            ... FAILED
test spec_testsuite::br_table               ... FAILED
test spec_testsuite::f64_bitwise            ... FAILED
test spec_testsuite::f32                    ... FAILED
test spec_testsuite::f32_cmp                ... FAILED
test spec_testsuite::fac                    ... ok
test spec_testsuite::float_literals         ... FAILED
test spec_testsuite::f64                    ... FAILED
test spec_testsuite::float_misc             ... FAILED
test spec_testsuite::forward                ... ok
test spec_testsuite::f64_cmp                ... FAILED
test spec_testsuite::func_ptrs              ... FAILED
test spec_testsuite::get_local              ... FAILED
test spec_testsuite::float_memory           ... ok
test spec_testsuite::globals                ... FAILED
test spec_testsuite::float_exprs            ... FAILED
test spec_testsuite::i64                    ... FAILED
test spec_testsuite::i32                    ... FAILED
test spec_testsuite::imports                ... FAILED
test spec_testsuite::inline_module          ... ok
test spec_testsuite::if_                    ... FAILED
test spec_testsuite::int_exprs              ... FAILED
test spec_testsuite::labels                 ... ok
test spec_testsuite::left_to_right          ... FAILED
test spec_testsuite::int_literals           ... ok
test spec_testsuite::linking                ... FAILED
test spec_testsuite::func                   ... FAILED
test spec_testsuite::memory_grow            ... FAILED
test spec_testsuite::loop_                  ... FAILED
test spec_testsuite::memory_redundancy      ... ok
test spec_testsuite::memory                 ... FAILED
test spec_testsuite::memory_trap            ... FAILED
test spec_testsuite::resizing               ... FAILED
test spec_testsuite::nop                    ... FAILED
test spec_testsuite::return_minimal         ... ok
test spec_testsuite::set_local              ... FAILED
test spec_testsuite::select                 ... FAILED
test spec_testsuite::stack                  ... FAILED
test spec_testsuite::start                  ... FAILED
test spec_testsuite::store_retval           ... ok
test spec_testsuite::skip_stack_guard_page  ... FAILED
test spec_testsuite::switch                 ... ok
test spec_testsuite::token                  ... ok
test spec_testsuite::tee_local              ... FAILED
test spec_testsuite::type_                  ... ok
test spec_testsuite::traps                  ... FAILED
test spec_testsuite::typecheck              ... ok
test spec_testsuite::unreached_invalid      ... ok
test spec_testsuite::unwind                 ... FAILED
test spec_testsuite::utf8_custom_section_id ... ok
test spec_testsuite::utf8_import_field      ... ok
test spec_testsuite::utf8_import_module     ... ok
test spec_testsuite::utf8_invalid_encoding  ... ok
test spec_testsuite::return_                ... ok
test spec_testsuite::unreachable            ... ok
test spec_testsuite::names                  ... FAILED

test result: FAILED. 31 passed; 45 failed; 0 ignored; 0 measured; 0 filtered out
```

## Getting involved

Our [issue tracker][issue tracker] is pretty barren right now since this is currently more-or-less a one-person project, but if you want to get involved jump into the [CraneStation Gitter room][cranestation-gitter] and someone can direct you to the right place. I wish I could say "the most useful thing you can do is play with it and open issues where you find problems" but until it passes the spec suite that won't be very helpful.

[cranestation-gitter]: https://gitter.im/CraneStation/Lobby
[issue tracker]: https://github.com/CraneStation/lightbeam/issues

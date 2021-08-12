;; make sure everything codegens correctly and has no cranelift verifier errors
(module
  (memory i64 1)
  (func (export "run")
    i64.const 0 i32.atomic.load drop
    i64.const 0 i64.atomic.load drop
    i64.const 0 i32.atomic.load8_u drop
    i64.const 0 i32.atomic.load16_u drop
    i64.const 0 i64.atomic.load8_u drop
    i64.const 0 i64.atomic.load16_u drop
    i64.const 0 i64.atomic.load32_u drop
    i64.const 0 i32.const 0 i32.atomic.store
    i64.const 0 i64.const 0 i64.atomic.store
    i64.const 0 i32.const 0 i32.atomic.store8
    i64.const 0 i32.const 0 i32.atomic.store16
    i64.const 0 i64.const 0 i64.atomic.store8
    i64.const 0 i64.const 0 i64.atomic.store16
    i64.const 0 i64.const 0 i64.atomic.store32
    i64.const 0 i32.const 0 i32.atomic.rmw.add drop
    i64.const 0 i64.const 0 i64.atomic.rmw.add drop
    i64.const 0 i32.const 0 i32.atomic.rmw8.add_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw16.add_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw8.add_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw16.add_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw32.add_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw.sub drop
    i64.const 0 i64.const 0 i64.atomic.rmw.sub drop
    i64.const 0 i32.const 0 i32.atomic.rmw8.sub_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw16.sub_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw8.sub_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw16.sub_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw32.sub_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw.and drop
    i64.const 0 i64.const 0 i64.atomic.rmw.and drop
    i64.const 0 i32.const 0 i32.atomic.rmw8.and_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw16.and_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw8.and_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw16.and_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw32.and_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw.or drop
    i64.const 0 i64.const 0 i64.atomic.rmw.or drop
    i64.const 0 i32.const 0 i32.atomic.rmw8.or_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw16.or_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw8.or_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw16.or_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw32.or_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw.xor drop
    i64.const 0 i64.const 0 i64.atomic.rmw.xor drop
    i64.const 0 i32.const 0 i32.atomic.rmw8.xor_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw16.xor_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw8.xor_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw16.xor_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw32.xor_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw.xchg drop
    i64.const 0 i64.const 0 i64.atomic.rmw.xchg drop
    i64.const 0 i32.const 0 i32.atomic.rmw8.xchg_u drop
    i64.const 0 i32.const 0 i32.atomic.rmw16.xchg_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw8.xchg_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw16.xchg_u drop
    i64.const 0 i64.const 0 i64.atomic.rmw32.xchg_u drop
    i64.const 0 i32.const 0 i32.const 0 i32.atomic.rmw.cmpxchg drop
    i64.const 0 i64.const 0 i64.const 0 i64.atomic.rmw.cmpxchg drop
    i64.const 0 i32.const 0 i32.const 0 i32.atomic.rmw8.cmpxchg_u drop
    i64.const 0 i32.const 0 i32.const 0 i32.atomic.rmw16.cmpxchg_u drop
    i64.const 0 i64.const 0 i64.const 0 i64.atomic.rmw8.cmpxchg_u drop
    i64.const 0 i64.const 0 i64.const 0 i64.atomic.rmw16.cmpxchg_u drop
    i64.const 0 i64.const 0 i64.const 0 i64.atomic.rmw32.cmpxchg_u drop
  )

  ;; these are unimplemented intrinsics that trap at runtime so just make sure
  ;; we can codegen instead of also testing execution.
  (func $just_validate_codegen
    i64.const 0 i32.const 0 memory.atomic.notify drop
    i64.const 0 i32.const 0 i64.const 0 memory.atomic.wait32 drop
    i64.const 0 i64.const 0 i64.const 0 memory.atomic.wait64 drop
  )
)

(assert_return (invoke "run"))

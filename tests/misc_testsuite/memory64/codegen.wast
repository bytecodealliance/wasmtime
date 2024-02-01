;; make sure everything codegens correctly and has no cranelift verifier errors
(module
  (memory i64 1)
  (func (export "run")
    i64.const 0 i64.const 0 i64.const 0 memory.copy
    i64.const 0 i32.const 0 i64.const 0 memory.fill
    i64.const 0 i32.const 0 i32.const 0 memory.init $seg
    memory.size drop
    i64.const 0 memory.grow drop

    i64.const 0 i32.load drop
    i64.const 0 i64.load drop
    i64.const 0 f32.load drop
    i64.const 0 f64.load drop
    i64.const 0 i32.load8_s drop
    i64.const 0 i32.load8_u drop
    i64.const 0 i32.load16_s drop
    i64.const 0 i32.load16_u drop
    i64.const 0 i64.load8_s drop
    i64.const 0 i64.load8_u drop
    i64.const 0 i64.load16_s drop
    i64.const 0 i64.load16_u drop
    i64.const 0 i64.load32_s drop
    i64.const 0 i64.load32_u drop
    i64.const 0 i32.const 0 i32.store
    i64.const 0 i64.const 0 i64.store
    i64.const 0 f32.const 0 f32.store
    i64.const 0 f64.const 0 f64.store
    i64.const 0 i32.const 0 i32.store8
    i64.const 0 i32.const 0 i32.store16
    i64.const 0 i64.const 0 i64.store8
    i64.const 0 i64.const 0 i64.store16
    i64.const 0 i64.const 0 i64.store32
  )

  (data $seg "..")
)
(assert_return (invoke "run"))

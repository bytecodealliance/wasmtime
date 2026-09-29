;;! threads = true

(module
  (memory (export "shared") 1 1 shared)

  (func (export "eq32") (result i32)
    (i32.atomic.store (i32.const 0) (i32.const 1))
    (memory.atomic.wait32 (i32.const 0) (i32.const 1) (i64.const 0)))

  (func (export "ne32") (result i32)
    (i32.atomic.store (i32.const 0) (i32.const 1))
    (memory.atomic.wait32 (i32.const 0) (i32.const 0x01000000) (i64.const 0)))

  (func (export "eq64") (result i32)
    (i64.atomic.store (i32.const 8) (i64.const 1))
    (memory.atomic.wait64 (i32.const 8) (i64.const 1) (i64.const 0)))

  (func (export "ne64") (result i32)
    (i64.atomic.store (i32.const 8) (i64.const 1))
    (memory.atomic.wait64 (i32.const 8) (i64.const 0x0100000000000000) (i64.const 0)))
)

(assert_return (invoke "eq32") (i32.const 2))
(assert_return (invoke "ne32") (i32.const 1))
(assert_return (invoke "eq64") (i32.const 2))
(assert_return (invoke "ne64") (i32.const 1))

;;! hogs_memory = true

(module
  (memory  0xffff)

  (func (export "grow") (param i32) (result i32)
    local.get 0
    memory.grow)
)

(assert_return (invoke "grow" (i32.const 0)) (i32.const 0xffff))
(assert_return (invoke "grow" (i32.const 1)) (i32.const 0xffff))
(assert_return (invoke "grow" (i32.const 0)) (i32.const 0x10000))
(assert_return (invoke "grow" (i32.const 1)) (i32.const -1))

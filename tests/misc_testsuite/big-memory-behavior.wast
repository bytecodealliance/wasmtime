;;! hogs_memory = true

(module
  (memory 0xffff)

  (func (export "grow") (param i32) (result i32)
    local.get 0
    memory.grow)

  (func (export "size") (result i32)
    memory.size)
)

(assert_return (invoke "grow" (i32.const 0)) (i32.const 0xffff))
(assert_return (invoke "size") (i32.const 0xffff))
(assert_return (invoke "grow" (i32.const 1)) (i32.const 0xffff))
(assert_return (invoke "size") (i32.const 0x10000))
(assert_return (invoke "grow" (i32.const 0)) (i32.const 0x10000))
(assert_return (invoke "grow" (i32.const 1)) (i32.const -1))

;; A data segment at a "negative" `i32` offset is still in bounds of a 4GiB
;; memory.
(module
  (memory 65536)
  (data (i32.const 0x8000_0000) "x"))

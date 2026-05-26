;;! memory64 = true
;;! bulk_memory = true
;;! multi_memory = true
;;! custom_page_sizes = true

(module
  (memory $m32 1)
  (func (export "fill32") (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m32
  )

  (memory $m64 i64 1)
  (func (export "fill64") (param i64 i64)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m64
  )

  (memory $m32p1 65536 (pagesize 1))
  (func (export "fill32p1") (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m32p1
  )

  (memory $m64p1 i64 65536 (pagesize 1))
  (func (export "fill64p1") (param i64 i64)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m64p1
  )

  (memory $empty 0 0)
  (func (export "fill-empty") (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $empty
  )
)

(assert_return (invoke "fill32" (i32.const 0) (i32.const 16)))
(assert_return (invoke "fill32" (i32.const 0) (i32.const 0)))
(assert_return (invoke "fill32" (i32.const 65536) (i32.const 0)))
(assert_trap (invoke "fill32" (i32.const 65536) (i32.const 1)) "out of bounds")
(assert_return (invoke "fill32" (i32.const 65535) (i32.const 1)))
(assert_trap (invoke "fill32" (i32.const 65535) (i32.const 2)) "out of bounds")
(assert_trap (invoke "fill32" (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "fill32" (i32.const 1) (i32.const -1)) "out of bounds")
(assert_trap (invoke "fill32" (i32.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "fill32" (i32.const -1) (i32.const 1)) "out of bounds")

(assert_return (invoke "fill64" (i64.const 0) (i64.const 16)))
(assert_return (invoke "fill64" (i64.const 0) (i64.const 0)))
(assert_return (invoke "fill64" (i64.const 65536) (i64.const 0)))
(assert_trap (invoke "fill64" (i64.const 65536) (i64.const 1)) "out of bounds")
(assert_return (invoke "fill64" (i64.const 65535) (i64.const 1)))
(assert_trap (invoke "fill64" (i64.const 65535) (i64.const 2)) "out of bounds")
(assert_trap (invoke "fill64" (i64.const 0) (i64.const -1)) "out of bounds")
(assert_trap (invoke "fill64" (i64.const 1) (i64.const -1)) "out of bounds")
(assert_trap (invoke "fill64" (i64.const -1) (i64.const 0)) "out of bounds")
(assert_trap (invoke "fill64" (i64.const -1) (i64.const 1)) "out of bounds")

(assert_return (invoke "fill32p1" (i32.const 0) (i32.const 16)))
(assert_return (invoke "fill32p1" (i32.const 0) (i32.const 0)))
(assert_return (invoke "fill32p1" (i32.const 65536) (i32.const 0)))
(assert_trap (invoke "fill32p1" (i32.const 65536) (i32.const 1)) "out of bounds")
(assert_return (invoke "fill32p1" (i32.const 65535) (i32.const 1)))
(assert_trap (invoke "fill32p1" (i32.const 65535) (i32.const 2)) "out of bounds")
(assert_trap (invoke "fill32p1" (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "fill32p1" (i32.const 1) (i32.const -1)) "out of bounds")
(assert_trap (invoke "fill32p1" (i32.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "fill32p1" (i32.const -1) (i32.const 1)) "out of bounds")

(assert_return (invoke "fill64p1" (i64.const 0) (i64.const 16)))
(assert_return (invoke "fill64p1" (i64.const 0) (i64.const 0)))
(assert_return (invoke "fill64p1" (i64.const 65536) (i64.const 0)))
(assert_trap (invoke "fill64p1" (i64.const 65536) (i64.const 1)) "out of bounds")
(assert_return (invoke "fill64p1" (i64.const 65535) (i64.const 1)))
(assert_trap (invoke "fill64p1" (i64.const 0) (i64.const -1)) "out of bounds")
(assert_trap (invoke "fill64p1" (i64.const 1) (i64.const -1)) "out of bounds")
(assert_trap (invoke "fill64p1" (i64.const 65535) (i64.const 2)) "out of bounds")
(assert_trap (invoke "fill64p1" (i64.const -1) (i64.const 0)) "out of bounds")
(assert_trap (invoke "fill64p1" (i64.const -1) (i64.const 1)) "out of bounds")

(assert_return (invoke "fill-empty" (i32.const 0) (i32.const 0)))
(assert_trap (invoke "fill-empty" (i32.const 0) (i32.const 1)) "out of bounds")
(assert_trap (invoke "fill-empty" (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "fill-empty" (i32.const 1) (i32.const -1)) "out of bounds")
(assert_trap (invoke "fill-empty" (i32.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "fill-empty" (i32.const -1) (i32.const 1)) "out of bounds")

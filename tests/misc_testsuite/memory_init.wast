;;! multi_memory = true
;;! bulk_memory = true
;;! memory64 = true

;; A test about the trapping behavior of `memory.init`

(module
  (memory $m32 1)
  (memory $m64 i64 1)

  (data $d "hi")
  (data $a (i32.const 0) "hi2")

  (func (export "init32-d") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.init $m32 $d)

  (func (export "init64-d") (param i64 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.init $m64 $d)

  (func (export "drop-d") data.drop $d)

  (func (export "init32-a") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.init $m32 $a)

  (func (export "init64-a") (param i64 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.init $m64 $a)
)

(assert_return (invoke "init32-d" (i32.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "init32-d" (i32.const 10) (i32.const 1) (i32.const 1)))
(assert_return (invoke "init32-d" (i32.const 100) (i32.const 0) (i32.const 2)))
(assert_return (invoke "init32-d" (i32.const 65536) (i32.const 2) (i32.const 0)))
(assert_return (invoke "init32-d" (i32.const 65535) (i32.const 1) (i32.const 1)))
(assert_return (invoke "init32-d" (i32.const 65534) (i32.const 0) (i32.const 2)))
(assert_trap (invoke "init32-d" (i32.const 0) (i32.const 0) (i32.const 3)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 0) (i32.const 3) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 65537) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const -1) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 1) (i32.const -1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 1) (i32.const 1) (i32.const -1)) "out of bounds")

(assert_return (invoke "init64-d" (i64.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "init64-d" (i64.const 10) (i32.const 1) (i32.const 1)))
(assert_return (invoke "init64-d" (i64.const 100) (i32.const 0) (i32.const 2)))
(assert_return (invoke "init64-d" (i64.const 65536) (i32.const 2) (i32.const 0)))
(assert_return (invoke "init64-d" (i64.const 65535) (i32.const 1) (i32.const 1)))
(assert_return (invoke "init64-d" (i64.const 65534) (i32.const 0) (i32.const 2)))
(assert_trap (invoke "init64-d" (i64.const 0) (i32.const 0) (i32.const 3)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 0) (i32.const 3) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 65537) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const -1) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 1) (i32.const -1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 1) (i32.const 1) (i32.const -1)) "out of bounds")

(assert_return (invoke "drop-d"))

(assert_return (invoke "init32-d" (i32.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "init32-d" (i32.const 65536) (i32.const 0) (i32.const 0)))
(assert_trap (invoke "init32-d" (i32.const 10) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 100) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 65536) (i32.const 2) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 65535) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 65534) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 0) (i32.const 0) (i32.const 3)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 0) (i32.const 3) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 65537) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const -1) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 1) (i32.const -1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-d" (i32.const 1) (i32.const 1) (i32.const -1)) "out of bounds")

(assert_return (invoke "init64-d" (i64.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "init64-d" (i64.const 65536) (i32.const 0) (i32.const 0)))
(assert_trap (invoke "init64-d" (i64.const 10) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 100) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 65536) (i32.const 2) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 65535) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 65534) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 0) (i32.const 0) (i32.const 3)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 0) (i32.const 3) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 65537) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const -1) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 1) (i32.const -1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-d" (i64.const 1) (i32.const 1) (i32.const -1)) "out of bounds")

(assert_return (invoke "init32-a" (i32.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "init32-a" (i32.const 65536) (i32.const 0) (i32.const 0)))
(assert_trap (invoke "init32-a" (i32.const 10) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 100) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 65536) (i32.const 2) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 65535) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 65534) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 0) (i32.const 0) (i32.const 3)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 0) (i32.const 3) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 65537) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const -1) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 1) (i32.const -1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init32-a" (i32.const 1) (i32.const 1) (i32.const -1)) "out of bounds")

(assert_return (invoke "init64-a" (i64.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "init64-a" (i64.const 65536) (i32.const 0) (i32.const 0)))
(assert_trap (invoke "init64-a" (i64.const 10) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 100) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 65536) (i32.const 2) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 65535) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 65534) (i32.const 0) (i32.const 2)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 0) (i32.const 0) (i32.const 3)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 0) (i32.const 3) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 65537) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const -1) (i32.const 1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 1) (i32.const -1) (i32.const 1)) "out of bounds")
(assert_trap (invoke "init64-a" (i64.const 1) (i32.const 1) (i32.const -1)) "out of bounds")

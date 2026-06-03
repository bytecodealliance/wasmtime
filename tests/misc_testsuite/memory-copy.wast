;;! bulk_memory = true
;;! multi_memory = true
;;! memory64 = true

(module
  (memory 1 1)
  (data 0 (i32.const 1000) "hello")
  (data 0 (i32.const 2000) "olleh")

  (func $is_char (param i32 i32) (result i32)
    local.get 0
    i32.load8_u
    local.get 1
    i32.eq)

  (func (export "is hello?") (param i32) (result i32)
    local.get 0
    i32.const 104 ;; 'h'
    call $is_char

    local.get 0
    i32.const 1
    i32.add
    i32.const 101 ;; 'e'
    call $is_char

    local.get 0
    i32.const 2
    i32.add
    i32.const 108 ;; 'l'
    call $is_char

    local.get 0
    i32.const 3
    i32.add
    i32.const 108 ;; 'l'
    call $is_char

    local.get 0
    i32.const 4
    i32.add
    i32.const 111 ;; 'o'
    call $is_char

    i32.and
    i32.and
    i32.and
    i32.and
  )

  (func (export "is olleh?") (param i32) (result i32)
    local.get 0
    i32.const 111 ;; 'o'
    call $is_char

    local.get 0
    i32.const 1
    i32.add
    i32.const 108 ;; 'l'
    call $is_char

    local.get 0
    i32.const 2
    i32.add
    i32.const 108 ;; 'l'
    call $is_char

    local.get 0
    i32.const 3
    i32.add
    i32.const 101 ;; 'e'
    call $is_char

    local.get 0
    i32.const 4
    i32.add
    i32.const 104 ;; 'h'
    call $is_char

    i32.and
    i32.and
    i32.and
    i32.and
  )

  (func (export "memory.copy") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy))

;; Our memory has our initial data in the right places.
(assert_return
  (invoke "is hello?" (i32.const 1000))
  (i32.const 1))
(assert_return
  (invoke "is olleh?" (i32.const 2000))
  (i32.const 1))

;; Non-overlapping memory copy with dst < src.
(invoke "memory.copy" (i32.const 500) (i32.const 1000) (i32.const 5))
(assert_return
  (invoke "is hello?" (i32.const 500))
  (i32.const 1))

;; Non-overlapping memory copy with dst > src.
(invoke "memory.copy" (i32.const 1500) (i32.const 1000) (i32.const 5))
(assert_return
  (invoke "is hello?" (i32.const 1500))
  (i32.const 1))

;; Overlapping memory copy with dst < src.
(invoke "memory.copy" (i32.const 1998) (i32.const 2000) (i32.const 5))
(assert_return
  (invoke "is olleh?" (i32.const 1998))
  (i32.const 1))

;; Overlapping memory copy with dst > src.
(invoke "memory.copy" (i32.const 2000) (i32.const 1998) (i32.const 5))
(assert_return
  (invoke "is olleh?" (i32.const 2000))
  (i32.const 1))

;; Overlapping memory copy with dst = src.
(invoke "memory.copy" (i32.const 2000) (i32.const 2000) (i32.const 5))
(assert_return
  (invoke "is olleh?" (i32.const 2000))
  (i32.const 1))

;; test trapping boundary behavior
(module
  (memory $m32_a 1)
  (memory $m32_b 1)
  (memory $m64_a i64 1)
  (memory $m64_b i64 1)

  (func (export "m32_to_same") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m32_a $m32_a
  )

  (func (export "m32_to_m32") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m32_b $m32_a
  )

  (func (export "m32_to_m64") (param i64 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m64_a $m32_a
  )

  (func (export "m64_to_same") (param i64 i64 i64)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m64_a $m64_a
  )

  (func (export "m64_to_m64") (param i64 i64 i64)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m64_b $m64_a
  )

  (func (export "m64_to_m32") (param i32 i64 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m32_a $m64_a
  )
)

(assert_return (invoke "m32_to_same" (i32.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "m32_to_same" (i32.const 0) (i32.const 0) (i32.const 65536)))
(assert_trap (invoke "m32_to_same" (i32.const 0) (i32.const 0) (i32.const 65537)) "out of bounds")
(assert_return (invoke "m32_to_same" (i32.const 100) (i32.const 200) (i32.const 65336)))
(assert_return (invoke "m32_to_same" (i32.const 200) (i32.const 100) (i32.const 65336)))
(assert_trap (invoke "m32_to_same" (i32.const 201) (i32.const 100) (i32.const 65336)) "out of bounds")
(assert_return (invoke "m32_to_same" (i32.const 200) (i32.const 101) (i32.const 65336)))
(assert_trap (invoke "m32_to_same" (i32.const 200) (i32.const 100) (i32.const 65337)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const -1) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const 0) (i32.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const 0) (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const 100) (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const 0) (i32.const 100) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const -1) (i32.const 0) (i32.const 100)) "out of bounds")
(assert_trap (invoke "m32_to_same" (i32.const 0) (i32.const -1) (i32.const 100)) "out of bounds")

(assert_return (invoke "m32_to_m32" (i32.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "m32_to_m32" (i32.const 0) (i32.const 0) (i32.const 65536)))
(assert_trap (invoke "m32_to_m32" (i32.const 0) (i32.const 0) (i32.const 65537)) "out of bounds")
(assert_return (invoke "m32_to_m32" (i32.const 100) (i32.const 200) (i32.const 65336)))
(assert_return (invoke "m32_to_m32" (i32.const 200) (i32.const 100) (i32.const 65336)))
(assert_trap (invoke "m32_to_m32" (i32.const 201) (i32.const 100) (i32.const 65336)) "out of bounds")
(assert_return (invoke "m32_to_m32" (i32.const 200) (i32.const 101) (i32.const 65336)))
(assert_trap (invoke "m32_to_m32" (i32.const 200) (i32.const 100) (i32.const 65337)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const -1) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const 0) (i32.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const 0) (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const 100) (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const 0) (i32.const 100) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const -1) (i32.const 0) (i32.const 100)) "out of bounds")
(assert_trap (invoke "m32_to_m32" (i32.const 0) (i32.const -1) (i32.const 100)) "out of bounds")

(assert_return (invoke "m32_to_m64" (i64.const 0) (i32.const 0) (i32.const 0)))
(assert_return (invoke "m32_to_m64" (i64.const 0) (i32.const 0) (i32.const 65536)))
(assert_trap (invoke "m32_to_m64" (i64.const 0) (i32.const 0) (i32.const 65537)) "out of bounds")
(assert_return (invoke "m32_to_m64" (i64.const 100) (i32.const 200) (i32.const 65336)))
(assert_return (invoke "m32_to_m64" (i64.const 200) (i32.const 100) (i32.const 65336)))
(assert_trap (invoke "m32_to_m64" (i64.const 201) (i32.const 100) (i32.const 65336)) "out of bounds")
(assert_return (invoke "m32_to_m64" (i64.const 200) (i32.const 101) (i32.const 65336)))
(assert_trap (invoke "m32_to_m64" (i64.const 200) (i32.const 100) (i32.const 65337)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const -1) (i32.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const 0) (i32.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const 0) (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const 100) (i32.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const 0) (i32.const 100) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const -1) (i32.const 0) (i32.const 100)) "out of bounds")
(assert_trap (invoke "m32_to_m64" (i64.const 0) (i32.const -1) (i32.const 100)) "out of bounds")

(assert_return (invoke "m64_to_same" (i64.const 0) (i64.const 0) (i64.const 0)))
(assert_return (invoke "m64_to_same" (i64.const 0) (i64.const 0) (i64.const 65536)))
(assert_trap (invoke "m64_to_same" (i64.const 0) (i64.const 0) (i64.const 65537)) "out of bounds")
(assert_return (invoke "m64_to_same" (i64.const 100) (i64.const 200) (i64.const 65336)))
(assert_return (invoke "m64_to_same" (i64.const 200) (i64.const 100) (i64.const 65336)))
(assert_trap (invoke "m64_to_same" (i64.const 201) (i64.const 100) (i64.const 65336)) "out of bounds")
(assert_return (invoke "m64_to_same" (i64.const 200) (i64.const 101) (i64.const 65336)))
(assert_trap (invoke "m64_to_same" (i64.const 200) (i64.const 100) (i64.const 65337)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const -1) (i64.const 0) (i64.const 0)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const 0) (i64.const -1) (i64.const 0)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const 0) (i64.const 0) (i64.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const 100) (i64.const 0) (i64.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const 0) (i64.const 100) (i64.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const -1) (i64.const 0) (i64.const 100)) "out of bounds")
(assert_trap (invoke "m64_to_same" (i64.const 0) (i64.const -1) (i64.const 100)) "out of bounds")

(assert_return (invoke "m64_to_m64" (i64.const 0) (i64.const 0) (i64.const 0)))
(assert_return (invoke "m64_to_m64" (i64.const 0) (i64.const 0) (i64.const 65536)))
(assert_trap (invoke "m64_to_m64" (i64.const 0) (i64.const 0) (i64.const 65537)) "out of bounds")
(assert_return (invoke "m64_to_m64" (i64.const 100) (i64.const 200) (i64.const 65336)))
(assert_return (invoke "m64_to_m64" (i64.const 200) (i64.const 100) (i64.const 65336)))
(assert_trap (invoke "m64_to_m64" (i64.const 201) (i64.const 100) (i64.const 65336)) "out of bounds")
(assert_return (invoke "m64_to_m64" (i64.const 200) (i64.const 101) (i64.const 65336)))
(assert_trap (invoke "m64_to_m64" (i64.const 200) (i64.const 100) (i64.const 65337)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const -1) (i64.const 0) (i64.const 0)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const 0) (i64.const -1) (i64.const 0)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const 0) (i64.const 0) (i64.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const 100) (i64.const 0) (i64.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const 0) (i64.const 100) (i64.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const -1) (i64.const 0) (i64.const 100)) "out of bounds")
(assert_trap (invoke "m64_to_m64" (i64.const 0) (i64.const -1) (i64.const 100)) "out of bounds")

(assert_return (invoke "m64_to_m32" (i32.const 0) (i64.const 0) (i32.const 0)))
(assert_return (invoke "m64_to_m32" (i32.const 0) (i64.const 0) (i32.const 65536)))
(assert_trap (invoke "m64_to_m32" (i32.const 0) (i64.const 0) (i32.const 65537)) "out of bounds")
(assert_return (invoke "m64_to_m32" (i32.const 100) (i64.const 200) (i32.const 65336)))
(assert_return (invoke "m64_to_m32" (i32.const 200) (i64.const 100) (i32.const 65336)))
(assert_trap (invoke "m64_to_m32" (i32.const 201) (i64.const 100) (i32.const 65336)) "out of bounds")
(assert_return (invoke "m64_to_m32" (i32.const 200) (i64.const 101) (i32.const 65336)))
(assert_trap (invoke "m64_to_m32" (i32.const 200) (i64.const 100) (i32.const 65337)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const -1) (i64.const 0) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const 0) (i64.const -1) (i32.const 0)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const 0) (i64.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const 100) (i64.const 0) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const 0) (i64.const 100) (i32.const -1)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const -1) (i64.const 0) (i32.const 100)) "out of bounds")
(assert_trap (invoke "m64_to_m32" (i32.const 0) (i64.const -1) (i32.const 100)) "out of bounds")

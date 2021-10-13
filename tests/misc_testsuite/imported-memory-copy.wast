(module $foreign
  (memory (export "mem") 1 1)
  (data 0 (i32.const 1000) "hello")
  (data 0 (i32.const 2000) "olleh"))

(register "foreign" $foreign)

(module
  (memory (import "foreign" "mem") 1 1)

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

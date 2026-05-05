;;! gc = true

;; Test br_on_cast and br_on_cast_fail with GC reference types.

(module
  (type $s  (sub (struct (field i32))))
  (type $s2 (sub $s (struct (field i32) (field i32))))
  (type $arr (array i8))

  (table 6 anyref)

  (func (export "init")
    ;; slot 0: null
    (table.set (i32.const 0) (ref.null any))
    ;; slot 1: i31ref
    (table.set (i32.const 1) (ref.i31 (i32.const 7)))
    ;; slot 2: struct $s
    (table.set (i32.const 2) (struct.new $s (i32.const 10)))
    ;; slot 3: struct $s2 (sub-type of $s)
    (table.set (i32.const 3) (struct.new $s2 (i32.const 20) (i32.const 30)))
    ;; slot 4: array
    (table.set (i32.const 4) (array.new_default $arr (i32.const 3)))
    ;; slot 5: null anyref
    (table.set (i32.const 5) (ref.null none))
  )

  ;; br_on_cast to (ref i31): branch if the value is an i31ref.
  (func (export "cast-to-i31") (param $i i32) (result i32)
    (block $l (result (ref i31))
      (br_on_cast $l anyref (ref i31) (table.get (local.get $i)))
      (return (i32.const -1))
    )
    (i31.get_u)
  )

  ;; br_on_cast to (ref struct): branch if the value is any struct.
  (func (export "cast-to-struct") (param $i i32) (result i32)
    (block $l (result (ref struct))
      (br_on_cast $l anyref (ref struct) (table.get (local.get $i)))
      (return (i32.const -1))
    )
    (drop)
    (i32.const 1)
  )

  ;; br_on_cast to concrete type $s2: branch only if it IS $s2.
  (func (export "cast-to-s2") (param $i i32) (result i32)
    (block $l (result (ref $s2))
      (br_on_cast $l anyref (ref $s2) (table.get (local.get $i)))
      (return (i32.const -1))
    )
    ;; read the second field (only $s2 has it)
    (struct.get $s2 1)
  )

  ;; br_on_cast to (ref array): branch if value is an array; use array.len.
  (func (export "cast-to-array-len") (param $i i32) (result i32)
    (block $l (result (ref array))
      (br_on_cast $l anyref (ref array) (table.get (local.get $i)))
      (return (i32.const -1))
    )
    (array.len)
  )

  ;; br_on_cast_fail: branch when the cast FAILS.
  ;; Returns 0 if cast succeeded (fell through to cast), -1 if cast failed (branch taken).
  (func (export "cast-fail-to-i31") (param $i i32) (result i32)
    (block $l (result anyref)
      (br_on_cast_fail $l anyref (ref i31) (table.get (local.get $i)))
      ;; cast succeeded: we have (ref i31) on the stack
      (drop)
      (return (i32.const 0))
    )
    ;; cast failed: we have anyref on the stack
    (drop)
    (i32.const -1)
  )

  ;; br_on_cast_fail to (ref struct)
  (func (export "cast-fail-to-struct") (param $i i32) (result i32)
    (block $l (result anyref)
      (br_on_cast_fail $l anyref (ref struct) (table.get (local.get $i)))
      (drop)
      (return (i32.const 0))
    )
    (drop)
    (i32.const -1)
  )
)

(invoke "init")

;; cast-to-i31: only slot 1 (i31ref) succeeds
(assert_return (invoke "cast-to-i31" (i32.const 0)) (i32.const -1)) ;; null -> fail
(assert_return (invoke "cast-to-i31" (i32.const 1)) (i32.const 7))  ;; i31(7) -> success
(assert_return (invoke "cast-to-i31" (i32.const 2)) (i32.const -1)) ;; struct -> fail
(assert_return (invoke "cast-to-i31" (i32.const 4)) (i32.const -1)) ;; array -> fail

;; cast-to-struct: slots 2 and 3 (structs) succeed
(assert_return (invoke "cast-to-struct" (i32.const 0)) (i32.const -1))
(assert_return (invoke "cast-to-struct" (i32.const 1)) (i32.const -1))
(assert_return (invoke "cast-to-struct" (i32.const 2)) (i32.const 1))
(assert_return (invoke "cast-to-struct" (i32.const 3)) (i32.const 1))
(assert_return (invoke "cast-to-struct" (i32.const 4)) (i32.const -1))

;; cast-to-s2: only slot 3 (the $s2 instance) succeeds
(assert_return (invoke "cast-to-s2" (i32.const 2)) (i32.const -1))
(assert_return (invoke "cast-to-s2" (i32.const 3)) (i32.const 30))

;; cast-to-array-len: only slot 4 (array of len 3) succeeds
(assert_return (invoke "cast-to-array-len" (i32.const 0)) (i32.const -1))
(assert_return (invoke "cast-to-array-len" (i32.const 4)) (i32.const 3))

;; cast-fail-to-i31: branch on failure means -1 when NOT i31
(assert_return (invoke "cast-fail-to-i31" (i32.const 0)) (i32.const -1))  ;; null -> fail
(assert_return (invoke "cast-fail-to-i31" (i32.const 1)) (i32.const 0))   ;; i31 -> success
(assert_return (invoke "cast-fail-to-i31" (i32.const 2)) (i32.const -1))  ;; struct -> fail

;; cast-fail-to-struct
(assert_return (invoke "cast-fail-to-struct" (i32.const 2)) (i32.const 0))  ;; struct -> success
(assert_return (invoke "cast-fail-to-struct" (i32.const 1)) (i32.const -1)) ;; i31 -> fail
(assert_return (invoke "cast-fail-to-struct" (i32.const 4)) (i32.const -1)) ;; array -> fail

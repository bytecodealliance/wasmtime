;;! gc = true

;; Test that our inline GC barriers detect `i31`s and don't attempt to actually
;; deref them or anything like that.

;; Nullable GC references.
(module
  (table $table 1 1 anyref)

  (func (export "get") (param i32) (result anyref)
    local.get 0
    table.get $table
  )

  (func $do_set (param i32 anyref)
    local.get 0
    local.get 1
    table.set $table
  )

  (func (export "set") (param i32 i32)
    local.get 0
    (ref.i31 local.get 1)
    call $do_set
  )
)

(assert_return (invoke "get" (i32.const 0)) (ref.null any))
(invoke "set" (i32.const 0) (i32.const 42))
(assert_return (invoke "get" (i32.const 0)) (ref.i31))

;; Non-nullable GC references.
(module
  (table $table 1 1 (ref any) (ref.i31 (i32.const 0)))

  (func (export "get") (param i32) (result (ref any))
    local.get 0
    table.get $table
  )

  (func $do_set (param i32 (ref any))
    local.get 0
    local.get 1
    table.set $table
  )

  (func (export "set") (param i32 i32)
    local.get 0
    (ref.i31 local.get 1)
    call $do_set
  )
)

(assert_return (invoke "get" (i32.const 0)) (ref.i31))
(invoke "set" (i32.const 0) (i32.const 42))
(assert_return (invoke "get" (i32.const 0)) (ref.i31))

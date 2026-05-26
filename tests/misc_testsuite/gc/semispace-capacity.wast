;;! gc = true

(module
  (type $big (struct
    (field i64) (field i64) (field i64) (field i64)
    (field i64) (field i64) (field i64) (field i64)
  ))

  (import "wasmtime" "gc" (func $gc))

  ;; Keep a reference alive across GC so objects must survive collection.
  (global $keep (mut (ref null $big)) (ref.null $big))

  (func (export "run")
    ;; Allocate a struct, keep it alive, and force GC repeatedly.
    (global.set $keep (struct.new $big
      (i64.const 1) (i64.const 2) (i64.const 3) (i64.const 4)
      (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 8)
    ))
    call $gc
    (global.set $keep (struct.new $big
      (i64.const 1) (i64.const 2) (i64.const 3) (i64.const 4)
      (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 8)
    ))
    call $gc
    (global.set $keep (struct.new $big
      (i64.const 1) (i64.const 2) (i64.const 3) (i64.const 4)
      (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 8)
    ))
    call $gc
    ;; Verify the last struct is still correct.
    (if (i64.ne (struct.get $big 0 (global.get $keep)) (i64.const 1))
      (then unreachable)
    )
  )
)

(assert_return (invoke "run"))

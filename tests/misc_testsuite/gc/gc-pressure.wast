;;! gc = true

(module
  (type $box (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $keep (ref null $box))
    (local $i i32)

    ;; Create the value we want to keep
    (local.set $keep (struct.new $box (i32.const 999)))

    ;; Allocate and discard 100 objects, forcing many GC cycles
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (i32.const 100)))
        (call $gc)
        (drop (struct.new $box (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)
      )
    )

    ;; Read back the kept value
    (struct.get $box 0 (local.get $keep))
  )
)

(assert_return (invoke "test") (i32.const 999))

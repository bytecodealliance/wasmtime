;;! gc = true

;; Test that the pointer to the collector's heap-data structure
;; (VMCopyingHeapData, etc.) is loaded from vmctx with `readonly` (and, for the
;; copying collector, also `can_move`) and that this is okay. This pointer is
;; set once at instantiation, before any Wasm code runs, and never changes
;; afterwards, so `readonly` is correct from the Wasm code's POV.

(module
  (type $s (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))

  (func $assert_eq (param i32 i32)
    (if (i32.eq (local.get 0) (local.get 1))
      (then (return)))
    unreachable
  )

  ;; Allocate, GC, allocate again, GC again, then read both objects. The
  ;; heap-data pointer is used internally for every allocation and GC; if it
  ;; were wrong, either the allocation would crash or the values would be
  ;; corrupted.
  (func (export "test")
    (local $a (ref null $s))
    (local $b (ref null $s))

    (local.set $a (struct.new $s (i32.const 111)))
    (call $gc)
    (local.set $b (struct.new $s (i32.const 222)))
    (call $gc)

    (call $assert_eq (struct.get $s 0 (local.get $a)) (i32.const 111))
    (call $assert_eq (struct.get $s 0 (local.get $b)) (i32.const 222))
  )
)

(assert_return (invoke "test"))

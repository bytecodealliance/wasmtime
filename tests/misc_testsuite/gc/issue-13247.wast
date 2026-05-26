;;! gc = true

;; Regression test for https://github.com/bytecodealliance/wasmtime/issues/13247
;;
;; The null collector's bump pointer starts at index 1 (index 0 is reserved
;; because `VMGcRef` uses `NonZeroU32`). The `allocated_bytes()` method was
;; returning the raw bump pointer value, reporting 1 byte allocated on a fresh
;; heap with no actual allocations. The GC heap starts with 0-byte capacity, so
;; triggering a GC before any allocations stored `last_post_gc_allocated_bytes =
;; 1`, and a subsequent allocation OOM hit `debug_assert!(1 <= 0)` inside
;; `should_collect_first`.
;;
;; The `struct.new` in `test` ensures the compiled module sets
;; `needs_gc_heap = true`, so the GC store is created during instantiation with
;; a 0-byte heap. The start function triggers a GC before any heap allocations.
;; After instantiation, `(ref.extern 1)` allocates an externref through the
;; runtime path (`retry_after_gc_async`), which is where the assertion fired.

(module
  (import "wasmtime" "gc" (func $gc))
  (type $s (struct))
  (start $init)
  (func $init
    call $gc
  )
  (func (export "test") (param externref)
    struct.new $s
    drop
  )
)

(invoke "test" (ref.extern 1))

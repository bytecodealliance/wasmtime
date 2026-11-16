;;! gc = true
;;! flags = "-O gc-heap-may-move=n"

;; Configure the GC heap as never moving so that the load instructions we emit
;; to read its base pointer are marked `readonly` and `can_move`.
;;
;; Even though we never construct a `GcCompiler` during Wasm-to-CLIF
;; translation for this module, it will still access the GC heap, and we should
;; therefore allocate the GC heap before any Wasm code starts running.
;; Otherwise, the Wasm code can read the dangling GC heap base pointer before the
;; GC heap is allocated, and, because the base pointer loads are marked
;; `readonly` and `can_move`, keep that dangling base pointer around for the
;; whole function.
;;
;; Note that the `externref` must be allocated by the host during the call to
;; `run`, rather than passed in as an argument, because passing it in as an
;; argument would allocate the GC heap before `run` is called.

(module
  (import "wasmtime" "new-externref" (func $new_externref (param i32) (result externref)))
  (func (export "run")
    loop
      (call $new_externref (i32.const 0))
      any.convert_extern
      ref.cast structref    ;; Should trap on this cast.
      br_on_null 0
      unreachable
    end
  )
)

(assert_trap (invoke "run") "cast failure")

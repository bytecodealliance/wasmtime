;; A component that uses Wasmtime's unsafe intrinsics, exposed via
;; `-C unsafe-intrinsics`.
;;
;; This only reads the address of the store's data and asserts that it is
;; non-null; it never dereferences that pointer, so it does not depend on the
;; layout of the `wasmtime` CLI's store data.
(component
  (import "unsafe-intrinsics" (instance $intrinsics
    (export "store-data-address" (func (result u64)))))

  (core func $store-data-address
    (canon lower (func $intrinsics "store-data-address")))

  (core module $m
    (import "" "store-data-address" (func $store-data-address (result i64)))
    (func (export "run") (result i32)
      (if (i64.eqz (call $store-data-address))
        (then unreachable))
      i32.const 0)
  )

  (core instance $i
    (instantiate $m
      (with "" (instance (export "store-data-address" (func $store-data-address))))))

  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))
)

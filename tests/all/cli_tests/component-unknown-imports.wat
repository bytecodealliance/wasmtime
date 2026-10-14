;; A component with an undefined import, to exercise
;; `-W unknown-imports-default`.
;;
;; The import returns a `u32`, which the component traps on unless it is the
;; default of zero. Without the flag this cannot be instantiated at all.
(component
  (import "unknown:pkg/iface" (instance $iface
    (export "f" (func (result u32)))))

  (alias export $iface "f" (func $f))
  (core func $f-lowered (canon lower (func $f)))

  (core module $m
    (import "" "f" (func $f (result i32)))
    (func (export "run") (result i32)
      (if (i32.ne (call $f) (i32.const 0))
        (then unreachable))
      i32.const 0)
  )

  (core instance $i
    (instantiate $m
      (with "" (instance (export "f" (func $f-lowered))))))

  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))
)

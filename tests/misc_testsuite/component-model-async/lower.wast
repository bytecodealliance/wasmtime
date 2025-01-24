;;! component_model_async = true

;; async lower
(component
  (import "host-echo-u32" (func $foo (param "p1" u32) (result u32)))
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core func $foo (canon lower (func $foo) async (memory $libc "memory")))
  (core module $m
    (func (import "" "foo") (param i32 i32) (result i32))
  )
  (core instance $i (instantiate $m (with "" (instance (export "foo" (func $foo))))))
)

;;! component_model_gc = true
;;! gc = true

;; Tests for the component model float types under the GC data model, where
;; `f32` and `f64` lower to core `f32` and `f64` values.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.
;;
;; The `wast` harness cannot pass a bare float to a component function: a
;; top-level `(f32.const ...)` always parses as a core value. So these tests
;; hand the value across as a bit pattern, reinterpret it inside wasm, send
;; the float through the adapter, and report whether it came back bit-exact.

;; `f32`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param f32) (result f32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" f32) (result f32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param f32) (result f32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param f32) (result f32)))
    ;; Round-trip the bit pattern and report whether it survived exactly.
    (func (export "g") (param i32) (result i32)
      local.get 0
      f32.reinterpret_i32
      call $f
      i32.reinterpret_f32
      local.get 0
      i32.eq)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s32) (result bool)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (s32.const 0x00000000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x80000000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x3fc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0xbfc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x00000001)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f7fffff)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f800000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0xff800000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7fc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f800001)) (bool.const true))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param f32) (result f32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" f32) (result f32)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param f32) (result f32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param f32) (result f32)))
    ;; Round-trip the bit pattern and report whether it survived exactly.
    (func (export "g") (param i32) (result i32)
      local.get 0
      f32.reinterpret_i32
      call $f
      i32.reinterpret_f32
      local.get 0
      i32.eq)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s32) (result bool)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (s32.const 0x00000000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x80000000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x3fc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0xbfc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x00000001)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f7fffff)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f800000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0xff800000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7fc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f800001)) (bool.const true))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param f32) (result f32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" f32) (result f32)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param f32) (result f32)))
    (memory (export "m") 1)
    ;; Round-trip the bit pattern and report whether it survived exactly.
    (func (export "g") (param i32) (result i32)
      local.get 0
      f32.reinterpret_i32
      call $f
      i32.reinterpret_f32
      local.get 0
      i32.eq)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s32) (result bool)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (s32.const 0x00000000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x80000000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x3fc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0xbfc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x00000001)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f7fffff)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f800000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0xff800000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7fc00000)) (bool.const true))
(assert_return (invoke "g" (s32.const 0x7f800001)) (bool.const true))

;; `f64`

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param f64) (result f64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" f64) (result f64)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param f64) (result f64)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param f64) (result f64)))
    ;; Round-trip the bit pattern and report whether it survived exactly.
    (func (export "g") (param i64) (result i32)
      local.get 0
      f64.reinterpret_i64
      call $f
      i64.reinterpret_f64
      local.get 0
      i64.eq)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s64) (result bool)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (s64.const 0x0000000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x8000000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x3ff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0xbff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x0000000000000001)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7fefffffffffffff)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff0000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0xfff0000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff0000000000001)) (bool.const true))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param f64) (result f64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" f64) (result f64)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param f64) (result f64)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param f64) (result f64)))
    ;; Round-trip the bit pattern and report whether it survived exactly.
    (func (export "g") (param i64) (result i32)
      local.get 0
      f64.reinterpret_i64
      call $f
      i64.reinterpret_f64
      local.get 0
      i64.eq)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s64) (result bool)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (s64.const 0x0000000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x8000000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x3ff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0xbff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x0000000000000001)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7fefffffffffffff)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff0000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0xfff0000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff0000000000001)) (bool.const true))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param f64) (result f64) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" f64) (result f64)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param f64) (result f64)))
    (memory (export "m") 1)
    ;; Round-trip the bit pattern and report whether it survived exactly.
    (func (export "g") (param i64) (result i32)
      local.get 0
      f64.reinterpret_i64
      call $f
      i64.reinterpret_f64
      local.get 0
      i64.eq)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" s64) (result bool)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (s64.const 0x0000000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x8000000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x3ff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0xbff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x0000000000000001)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7fefffffffffffff)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff0000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0xfff0000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff8000000000000)) (bool.const true))
(assert_return (invoke "g" (s64.const 0x7ff0000000000001)) (bool.const true))

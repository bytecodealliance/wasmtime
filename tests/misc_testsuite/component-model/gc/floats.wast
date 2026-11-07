;;! component_model_gc = true
;;! gc = true

;; Tests for the component model float types under the GC data model, where
;; `f32` and `f64` lower to core `f32` and `f64` values.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.

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
    (func (export "g") (param f32) (result f32)
      local.get 0
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" f32) (result f32)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (f32.const 0.0)) (f32.const 0.0))
(assert_return (invoke "g" (f32.const -0.0)) (f32.const -0.0))
(assert_return (invoke "g" (f32.const 1.5)) (f32.const 1.5))
(assert_return (invoke "g" (f32.const -1.5)) (f32.const -1.5))
(assert_return (invoke "g" (f32.const 0x1p-149)) (f32.const 0x1p-149))
(assert_return (invoke "g" (f32.const 0x1.fffffep+127)) (f32.const 0x1.fffffep+127))
(assert_return (invoke "g" (f32.const inf)) (f32.const inf))
(assert_return (invoke "g" (f32.const -inf)) (f32.const -inf))
(assert_return (invoke "g" (f32.const nan:0x400000)) (f32.const nan:0x400000))
(assert_return (invoke "g" (f32.const nan:0x1)) (f32.const nan:0x1))

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
    (func (export "g") (param f32) (result f32)
      local.get 0
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" f32) (result f32)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (f32.const 0.0)) (f32.const 0.0))
(assert_return (invoke "g" (f32.const -0.0)) (f32.const -0.0))
(assert_return (invoke "g" (f32.const 1.5)) (f32.const 1.5))
(assert_return (invoke "g" (f32.const -1.5)) (f32.const -1.5))
(assert_return (invoke "g" (f32.const 0x1p-149)) (f32.const 0x1p-149))
(assert_return (invoke "g" (f32.const 0x1.fffffep+127)) (f32.const 0x1.fffffep+127))
(assert_return (invoke "g" (f32.const inf)) (f32.const inf))
(assert_return (invoke "g" (f32.const -inf)) (f32.const -inf))
(assert_return (invoke "g" (f32.const nan:0x400000)) (f32.const nan:0x400000))
(assert_return (invoke "g" (f32.const nan:0x1)) (f32.const nan:0x1))

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
    (func (export "g") (param f32) (result f32)
      local.get 0
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" f32) (result f32)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (f32.const 0.0)) (f32.const 0.0))
(assert_return (invoke "g" (f32.const -0.0)) (f32.const -0.0))
(assert_return (invoke "g" (f32.const 1.5)) (f32.const 1.5))
(assert_return (invoke "g" (f32.const -1.5)) (f32.const -1.5))
(assert_return (invoke "g" (f32.const 0x1p-149)) (f32.const 0x1p-149))
(assert_return (invoke "g" (f32.const 0x1.fffffep+127)) (f32.const 0x1.fffffep+127))
(assert_return (invoke "g" (f32.const inf)) (f32.const inf))
(assert_return (invoke "g" (f32.const -inf)) (f32.const -inf))
(assert_return (invoke "g" (f32.const nan:0x400000)) (f32.const nan:0x400000))
(assert_return (invoke "g" (f32.const nan:0x1)) (f32.const nan:0x1))

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
    (func (export "g") (param f64) (result f64)
      local.get 0
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" f64) (result f64)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (f64.const 0.0)) (f64.const 0.0))
(assert_return (invoke "g" (f64.const -0.0)) (f64.const -0.0))
(assert_return (invoke "g" (f64.const 1.5)) (f64.const 1.5))
(assert_return (invoke "g" (f64.const -1.5)) (f64.const -1.5))
(assert_return (invoke "g" (f64.const 0x1p-1074)) (f64.const 0x1p-1074))
(assert_return
  (invoke "g" (f64.const 0x1.fffffffffffffp+1023))
  (f64.const 0x1.fffffffffffffp+1023))
(assert_return (invoke "g" (f64.const inf)) (f64.const inf))
(assert_return (invoke "g" (f64.const -inf)) (f64.const -inf))
(assert_return (invoke "g" (f64.const nan:0x8000000000000)) (f64.const nan:0x8000000000000))
(assert_return (invoke "g" (f64.const nan:0x1)) (f64.const nan:0x1))

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
    (func (export "g") (param f64) (result f64)
      local.get 0
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" f64) (result f64)
    (canon lift (core func $i "g") gc))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (f64.const 0.0)) (f64.const 0.0))
(assert_return (invoke "g" (f64.const -0.0)) (f64.const -0.0))
(assert_return (invoke "g" (f64.const 1.5)) (f64.const 1.5))
(assert_return (invoke "g" (f64.const -1.5)) (f64.const -1.5))
(assert_return (invoke "g" (f64.const 0x1p-1074)) (f64.const 0x1p-1074))
(assert_return
  (invoke "g" (f64.const 0x1.fffffffffffffp+1023))
  (f64.const 0x1.fffffffffffffp+1023))
(assert_return (invoke "g" (f64.const inf)) (f64.const inf))
(assert_return (invoke "g" (f64.const -inf)) (f64.const -inf))
(assert_return (invoke "g" (f64.const nan:0x8000000000000)) (f64.const nan:0x8000000000000))
(assert_return (invoke "g" (f64.const nan:0x1)) (f64.const nan:0x1))

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
    (func (export "g") (param f64) (result f64)
      local.get 0
      call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" f64) (result f64)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

;; +/-0.0, +/-1.5, the smallest subnormal, the largest finite value,
;; +/-inf, a quiet NaN, and a signaling NaN.
(assert_return (invoke "g" (f64.const 0.0)) (f64.const 0.0))
(assert_return (invoke "g" (f64.const -0.0)) (f64.const -0.0))
(assert_return (invoke "g" (f64.const 1.5)) (f64.const 1.5))
(assert_return (invoke "g" (f64.const -1.5)) (f64.const -1.5))
(assert_return (invoke "g" (f64.const 0x1p-1074)) (f64.const 0x1p-1074))
(assert_return
  (invoke "g" (f64.const 0x1.fffffffffffffp+1023))
  (f64.const 0x1.fffffffffffffp+1023))
(assert_return (invoke "g" (f64.const inf)) (f64.const inf))
(assert_return (invoke "g" (f64.const -inf)) (f64.const -inf))
(assert_return (invoke "g" (f64.const nan:0x8000000000000)) (f64.const nan:0x8000000000000))
(assert_return (invoke "g" (f64.const nan:0x1)) (f64.const nan:0x1))

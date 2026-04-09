;;! simd = true

(module
  (memory 1)
  (func (export "test") (param i32) (result v128)
    (f64x2.splat (f64.load (local.get 0)))))

(assert_return (invoke "test" (i32.const 65528)) (v128.const f64x2 0 0))

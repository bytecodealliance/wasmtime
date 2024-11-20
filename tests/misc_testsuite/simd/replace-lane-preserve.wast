;;! simd = true

;; originally from #3216
(module
  (func (result i64)
    v128.const i64x2 -1 1
    global.get 0
    f64x2.replace_lane 0
    i64x2.extract_lane 1
  )
  (global f64 (f64.const 1))
  (export "" (func 0)))

(assert_return (invoke "") (i64.const 1))

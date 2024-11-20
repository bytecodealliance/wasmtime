;;! simd = true

 (module
  (func (result i32)
    global.get 0
    v128.any_true
  )
  (global (;0;) (mut v128) v128.const i64x2 0 0)
)


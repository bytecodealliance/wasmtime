(module
  (memory 1)
  (func (export "ceil") (param i32) (result f64)
    local.get 0
    f64.load
    f64.ceil
    return)
  (func (export "trunc") (param i32) (result f64)
    local.get 0
    f64.load
    f64.trunc
    return)
  (func (export "floor") (param i32) (result f64)
    local.get 0
    f64.load
    f64.floor
    return)
  (func (export "nearest") (param i32) (result f64)
    local.get 0
    f64.load
    f64.nearest
    return)
)

(assert_return (invoke "ceil" (i32.const 0xfff8)) (f64.const 0))
(assert_return (invoke "trunc" (i32.const 0xfff8)) (f64.const 0))
(assert_return (invoke "floor" (i32.const 0xfff8)) (f64.const 0))
(assert_return (invoke "nearest" (i32.const 0xfff8)) (f64.const 0))

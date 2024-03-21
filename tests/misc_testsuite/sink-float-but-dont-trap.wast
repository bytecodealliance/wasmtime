(module
  (memory 1)

  ;; make sure that the sunk load here doesn't try to load past the end of
  ;; memory.
  (func (export "select-with-sink") (param i32) (result f64)
    local.get 0
    f64.load
    f64.const 1
    local.get 0
    select
    return)

  ;; same as above but with a slightly different codegen pattern.
  (func (export "select-with-fcmp-and-sink") (param i32 f64 f64) (result f64)
    local.get 0
    f64.load
    f64.const 1
    local.get 1
    local.get 2
    f64.ne
    select
    return)

  ;; Same as the above two but the order of operands to the `select` are
  ;; swapped.
  (func (export "select-with-sink-other-way") (param i32) (result f64)
    f64.const 1
    local.get 0
    f64.load
    local.get 0
    select
    return)
  (func (export "select-with-fcmp-and-sink-other-way") (param i32 f64 f64) (result f64)
    f64.const 1
    local.get 0
    f64.load
    local.get 1
    local.get 2
    f64.ne
    select
    return)
)

(assert_return (invoke "select-with-sink" (i32.const 0xfff8)) (f64.const 0))
(assert_return (invoke "select-with-fcmp-and-sink" (i32.const 0xfff8) (f64.const 0) (f64.const 0)) (f64.const 1))

(assert_trap (invoke "select-with-sink" (i32.const 0xfff9)) "out of bounds")
(assert_trap (invoke "select-with-fcmp-and-sink" (i32.const 0xfff9) (f64.const 0) (f64.const 0)) "out of bounds")
(assert_trap (invoke "select-with-sink-other-way" (i32.const 0xfff9)) "out of bounds")
(assert_trap (invoke "select-with-fcmp-and-sink-other-way" (i32.const 0xfff9) (f64.const 0) (f64.const 0)) "out of bounds")

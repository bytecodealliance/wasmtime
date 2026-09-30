;;! tail_call = true

(module
  (func (export "callee") (param i32 i32 i32 i32 i32) (result i32)
    local.get 0))
(register "m")

(module
  (import "m" "callee" (func $callee (param i32 i32 i32 i32 i32) (result i32)))
  (func (export "run") (param i32) (result i32)
    local.get 0
    i32.const 2
    i32.const 3
    i32.const 4
    i32.const 5
    return_call $callee))

(assert_return (invoke "run" (i32.const 42)) (i32.const 42))

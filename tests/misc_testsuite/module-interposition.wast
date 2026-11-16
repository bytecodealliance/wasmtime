;; Each module imports `"instance" "export"` and is then registered under the
;; same name, shadowing the previous one, so every call goes through the whole
;; chain: 7 * 2^4 = 112.

(module (func (export "export") (result i32) (i32.const 7)))
(register "instance")

(module
  (import "instance" "export" (func (result i32)))
  (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2))))
(register "instance")

(module
  (import "instance" "export" (func (result i32)))
  (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2))))
(register "instance")

(module
  (import "instance" "export" (func (result i32)))
  (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2))))
(register "instance")

(module
  (import "instance" "export" (func (result i32)))
  (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2))))

(assert_return (invoke "export") (i32.const 112))

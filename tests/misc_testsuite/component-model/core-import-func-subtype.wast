;;! gc = true

;; A core instance inside a component may satisfy a function import of type
;; `$super` with a function whose type is a declared subtype of `$super`.
(component
  (core module $a
    (type $super (sub (func (result i32))))
    (type $sub (sub $super (func (result i32))))
    (func (export "f") (type $sub) i32.const 42))
  (core module $b
    (type $super (sub (func (result i32))))
    (type $g (func (result i32)))
    (import "a" "f" (func $f (type $super)))
    (func (export "g") (type $g) call $f))
  (core instance $a (instantiate $a))
  (core instance $b (instantiate $b (with "a" (instance $a))))
  (func (export "g") (result s32) (canon lift (core func $b "g"))))

(assert_return (invoke "g") (s32.const 42))

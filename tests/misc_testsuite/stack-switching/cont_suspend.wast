;;! stack_switching = true
;; Small continuation resume test
;; expected output:
;; 1 : i32
;; 2 : i32
;; 3 : i32
(module
  (func $print (import "spectest" "print_i32") (param i32) (result))
  (type $ft (func))
  (type $ct (cont $ft))
  (tag $h)
  (func $f (export "f")
    (suspend $h)
    (call $print (i32.const 2)))
  (func (export "run") (result i32)
    (call $print (i32.const 1))
    (block $on_h (result (ref $ct))
      (resume $ct (on $h $on_h)
                  (cont.new $ct (ref.func $f)))
      (unreachable))
    (drop)
    (call $print (i32.const 3))
    (return (i32.const 42)))
)

(assert_return (invoke "run") (i32.const 42))

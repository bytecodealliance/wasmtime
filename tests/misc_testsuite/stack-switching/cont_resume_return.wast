;;! stack_switching = true
(module
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (func $g (result i32)
    (i32.const 42))
  (elem declare func $g)

  (func $f (export "f") (result i32)
    (resume $ct (cont.new $ct (ref.func $g))))
)

(assert_return (invoke "f") (i32.const 42))
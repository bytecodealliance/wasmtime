;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (global $i (mut i32) (i32.const 0))

  (func $g
    (global.set $i (i32.const 42)))
  (elem declare func $g)

  (func $f (export "f") (result i32)
    (global.set $i (i32.const 99))
    (resume $ct (cont.new $ct (ref.func $g)))
    (global.get $i))
)

(assert_return (invoke "f") (i32.const 42))
;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (func $noop)
  (elem declare func $noop)

  (func $make-cont (result (ref $ct))
     (cont.new $ct (ref.func $noop)))

  (func $f (export "f") (result i32)
     (call $make-cont)
     (ref.is_null))
)

(assert_return (invoke "f") (i32.const 0))
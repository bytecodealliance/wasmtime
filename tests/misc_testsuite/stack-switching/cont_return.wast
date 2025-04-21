;;! stack_switching = true
;; Test returning value from continuation function without any suspending

(module

  (type $g_type (func (result i32)))
  (type $g_ct (cont $g_type))

  (func $g (result i32)
    (i32.const 100))
  (elem declare func $g)

  (func $f (export "f") (result i32)
    (resume $g_ct
      (cont.new $g_ct (ref.func $g)))))

(assert_return (invoke "f") (i32.const 100))

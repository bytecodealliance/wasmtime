;;! gc = true
;;! flags = "-Ogc-heap-may-move=n -Ogc-heap-reservation=0"

(module
  (type $a (array (mut i8)))

  (global $g (mut (ref $a)) (array.new_default $a (i32.const 12)))

  (func (export "array_get_nth") (param $p i32) (result i32)
    (array.get_u $a (global.get $g) (local.get $p))
  )
)

(assert_return (invoke "array_get_nth" (i32.const 0)) (i32.const 0))

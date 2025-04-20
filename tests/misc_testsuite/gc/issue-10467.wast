;;! gc = true

(module
  (type $string (array (mut i8)))
  (func (export "f")
    (local $s (ref $string))
    (local.set $s (array.new_default $string (i32.const 1)))
    (array.fill $string
      (local.get $s)
      (i32.const 0)
      (i32.const 32)
      (i32.const 1))
    (if (i32.ne (array.get_u $string (local.get $s) (i32.const 0))
                (i32.const 32))
      (then
        (unreachable)))
  )
)

(assert_return (invoke "f"))

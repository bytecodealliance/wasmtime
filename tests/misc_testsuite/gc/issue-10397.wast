;;! gc = true

(module
  (type $func (func))
  (type $array (array (mut i32)))
  (type $struct (sub (struct (field $field (ref $func)))))

  (elem func $nop)
  (func $nop)

  (func (export "")
    (local $local_array (ref $array))
    (local $local_struct (ref $struct))
    (local $i i32)

    (local.set $local_struct (struct.new $struct (ref.func $nop)))

    (loop $outer
      (local.set $local_array (array.new $array (i32.const 0) (i32.const 1)))

      (loop $inner
        (array.set $array (ref.cast (ref $array) (local.get $local_array))
                          (i32.const 0)
                          (i32.const 1))
        (br_if $inner (i32.const 0))
      )

      (call_ref $func (struct.get $struct $field (local.get $local_struct)))

      (if (i32.gt_u (local.get $i) (i32.shl (i32.const 1) (i32.const 14)))
        (then (return)))

      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $outer)
    )
  )
)

(assert_return (invoke ""))

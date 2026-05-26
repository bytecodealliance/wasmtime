(component
  (component $A
    (core module $m
      (global $cnt (mut i32) (i32.const 0))
      (func (export "inc")
        global.get $cnt
        i32.const 1
        i32.add
        global.set $cnt
      )

      (func (export "get") (result i32) global.get $cnt)
    )
    (core instance $i (instantiate $m))
    (func (export "inc") (canon lift (core func $i "inc")))
    (func (export "get") (result u32) (canon lift (core func $i "get")))
  )
  (component $B
    (import "inc" (func $inc))
    (component $c1
      (import "inc" (func $inc))
      (core func $inc_lower (canon lower (func $inc)))
      (core module $m
          (import "" "" (func $inc))
          (start $inc)
      )
      (core instance (instantiate $m (with "" (instance (export "" (func $inc_lower))))))
      (core instance (instantiate $m (with "" (instance (export "" (func $inc_lower))))))
    )
    (component $c2
      (import "inc" (func $inc))
      (instance (instantiate $c1 (with "inc" (func $inc))))
      (instance (instantiate $c1 (with "inc" (func $inc))))
    )
    (component $c3
      (import "inc" (func $inc))
      (instance (instantiate $c2 (with "inc" (func $inc))))
      (instance (instantiate $c2 (with "inc" (func $inc))))
    )
    (component $c4
      (import "inc" (func $inc))
      (instance (instantiate $c3 (with "inc" (func $inc))))
      (instance (instantiate $c3 (with "inc" (func $inc))))
    )

    (instance (instantiate $c4 (with "inc" (func $inc))))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "inc" (func $a "inc"))))
  (export "get" (func $a "get"))

)

(assert_return (invoke "get") (u32.const 16))

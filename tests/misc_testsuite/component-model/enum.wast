(component
  (type $a (enum "a" "b" "c"))
  (type $func_ty (func (param "x" $a)))

  (component $c1
    (import "a" (type $a' (eq $a)))

    (core module $m1
      (func (export "f") (result i32)
        (i32.const 0))
      (func (export "g") (result i32)
        (i32.const -1)))

    (core instance $ci1 (instantiate $m1))

    (func (export "f") (result $a') (canon lift (core func $ci1 "f")))
    (func (export "g") (result $a') (canon lift (core func $ci1 "g")))
  )

  (component $c2
    (import "a" (type $a' (eq $a)))
    (import "f" (func $f (result $a')))
    (import "g" (func $g (result $a')))

    (core func $f' (canon lower (func $f)))
    (core func $g' (canon lower (func $g)))

    (core module $m2
      (import "" "f" (func (result i32)))
      (import "" "g" (func (result i32)))
      (func (export "f") (call 0) (drop))
      (func (export "g") (call 1) (drop)))

    (core instance $ci2
      (instantiate $m2 (with "" (instance (export "f" (func $f'))
                                          (export "g" (func $g'))))))

    (func (export "f") (canon lift (core func $ci2 "f")))
    (func (export "g") (canon lift (core func $ci2 "g")))
  )

  (instance $i1 (instantiate $c1 (with "a" (type $a))))
  (instance $i2 (instantiate $c2
                  (with "a" (type $a))
                  (with "f" (func $i1 "f"))
                  (with "g" (func $i1 "g"))))

  (export "f" (func $i2 "f"))
  (export "g" (func $i2 "g"))
)

(assert_return (invoke "f"))
(assert_trap (invoke "g") "unreachable")

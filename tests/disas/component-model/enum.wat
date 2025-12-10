;;! target = "riscv64"
;;! test = 'optimize'
;;! filter = 'wasm[2]--function[1]'

(component
  (type $a (enum "a" "b" "c"))
  (type $func_ty (func (param "x" $a)))

  (component $c1
    (import "a" (type $a' (eq $a)))
    (core module $m1
      (func (export "f") (result i32)
        (i32.const 0)))
    (core instance $ci1 (instantiate $m1))
    (func (export "f") (result $a') (canon lift (core func $ci1 "f"))))

  (component $c2
    (import "a" (type $a' (eq $a)))
    (import "f" (func $f (result $a')))
    (core func $g (canon lower (func $f)))
    (core module $m2
      (import "" "f" (func (result i32)))
      (func (export "f") (result i32) (call 0)))
    (core instance $ci2
      (instantiate $m2 (with "" (instance (export "f" (func $g))))))
    (func (export "f") (result $a') (canon lift (core func $ci2 "f"))))

  (instance $i1 (instantiate $c1 (with "a" (type $a))))
  (instance $i2 (instantiate $c2
                  (with "a" (type $a))
                  (with "f" (func $i1 "f"))))
)


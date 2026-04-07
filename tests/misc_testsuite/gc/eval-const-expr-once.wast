;;! gc = true
;;! bulk_memory = true

;; Test that we only evaluate a const expression once, even if it is referenced
;; by e.g. `array.new_elem` many times.
(module
  (type $arr (array (mut arrayref)))

  (elem $elem arrayref
    (item (array.new_default $arr (i32.const 0))))

  (func (export "run") (result i32)
    (local $a (ref null $arr))
    (local $b (ref null $arr))

    (local.set $a (array.new_elem $arr $elem (i32.const 0) (i32.const 1)))
    (local.set $b (array.new_elem $arr $elem (i32.const 0) (i32.const 1)))

    (ref.eq (array.get $arr (local.get $a) (i32.const 0))
            (array.get $arr (local.get $b) (i32.const 0)))
  )
)

(assert_return (invoke "run") (i32.const 1))

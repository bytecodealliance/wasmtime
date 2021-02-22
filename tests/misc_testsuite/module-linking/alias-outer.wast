(module $a
  (module $m1)
  (module $b
    (module $m2)
    (module $c
      (instance (instantiate (module outer $a $m1)))
      (instance (instantiate (module outer $b $m2)))
    )
    (instance (instantiate $c))
  )
  (instance (instantiate $b))
)

(module $a
  (module (export "m"))
)

(module $PARENT
  (import "a" "m" (module $b))
  (module $c
    (module $d
      (instance (instantiate (module outer $PARENT $b)))
    )
    (instance (instantiate $d))
  )
  (instance (instantiate $c))
)

;; Instantiate `$b` here below twice with two different imports. Ensure the
;; exported modules close over the captured state correctly to ensure that we
;; get the right functions.
(module $a
  (module $b (export "close_over_imports")
    (import "m" (module $m (export "f" (func (result i32)))))
    (module (export "m")
      (instance $a (instantiate (module outer $b $m)))
      (func (export "f") (result i32)
        call (func $a "f"))
    )
  )
)

(module
  (import "a" "close_over_imports" (module $m0
    (import "m" (module (export "f" (func (result i32)))))
    (export "m" (module (export "f" (func (result i32)))))
  ))

  (module $m1
    (func (export "f") (result i32)
      i32.const 0))
  (instance $m_g1 (instantiate $m0 (import "m" (module $m1))))
  (instance $g1 (instantiate (module $m_g1 "m")))
  (module $m2
    (func (export "f") (result i32)
      i32.const 1))
  (instance $m_g2 (instantiate $m0 (import "m" (module $m2))))
  (instance $g2 (instantiate (module $m_g2 "m")))

  (func (export "get1") (result i32)
    call (func $g1 "f"))
  (func (export "get2") (result i32)
    call (func $g2 "f"))
)

(assert_return (invoke "get1") (i32.const 0))
(assert_return (invoke "get2") (i32.const 1))

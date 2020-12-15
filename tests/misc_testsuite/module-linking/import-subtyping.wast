;; subsets of imports
(module $a
  (module (export "m")
    (func (export ""))
    (func (export "a"))
    (global (export "b") i32 (i32.const 0))
  )
)

(module
  (import "a" "m" (module))
  (import "a" "m" (module (export "" (func))))
  (import "a" "m" (module (export "a" (func))))
  (import "a" "m" (module (export "b" (global i32))))
  (import "a" "m" (module
    (export "" (func))
    (export "a" (func))
  ))
  (import "a" "m" (module
    (export "a" (func))
    (export "" (func))
  ))
  (import "a" "m" (module
    (export "a" (func))
    (export "" (func))
    (export "b" (global i32))
  ))
  (import "a" "m" (module
    (export "b" (global i32))
    (export "a" (func))
    (export "" (func))
  ))
)

;; functions
(module $a
  (module (export "m")
    (func (export ""))))

(module
  (import "a" "m" (module))
  (import "a" "m" (module (export "" (func))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func (param i32))))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func (result i32))))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global i32)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "module types incompatible")

(module $a
  (module (export "m")
    (global (export "") i32 (i32.const 0))))

;; globals
(module
  (import "a" "m" (module))
  (import "a" "m" (module (export "" (global i32))))
)
(assert_unlinkable
  (module
    (import "a" "m" (module (export "" (global (mut i32)))))
  )
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "module types incompatible")

;; tables
(module $a
  (module (export "m")
    (table (export "") 1 funcref)
    (table (export "max") 1 10 funcref)
  )
)
(module
  (import "a" "m" (module))
  (import "a" "m" (module (export "" (table 1 funcref))))
  (import "a" "m" (module (export "" (table 0 funcref))))
  (import "a" "m" (module (export "max" (table 1 10 funcref))))
  (import "a" "m" (module (export "max" (table 0 10 funcref))))
  (import "a" "m" (module (export "max" (table 0 11 funcref))))
  (import "a" "m" (module (export "max" (table 0 funcref))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 2 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 10 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (table 2 10 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (table 1 9 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "module types incompatible")

;; memories
(module $a
  (module (export "m")
    (memory (export "") 1)
    (memory (export "max") 1 10)
  )
)
(module
  (import "a" "m" (module))
  (import "a" "m" (module (export "" (memory 1))))
  (import "a" "m" (module (export "" (memory 0))))
  (import "a" "m" (module (export "max" (memory 1 10))))
  (import "a" "m" (module (export "max" (memory 0 10))))
  (import "a" "m" (module (export "max" (memory 0 11))))
  (import "a" "m" (module (export "max" (memory 0))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 2)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1 10)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (memory 2 10)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (memory 2)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "module types incompatible")

;; modules
(module $a
  (module (export "m")
    ;; export nothing
    (module (export "a"))
    ;; export one thing
    (module (export "b")
      (func (export ""))
    )
    ;; export a mixture
    (module (export "c")
      (func (export "a"))
      (func (export "b") (result i32)
        i32.const 0)
      (global (export "c") i32 (i32.const 0))
    )
    ;; import one thing
    (module (export "d")
      (import "" (func))
    )
    ;; import a mixture
    (module (export "e")
      (import "" (func))
      (import "" (func))
      (import "" (global i32))
    )
  )
)
(module
  (import "a" "m" (module))
  (import "a" "m" (module (export "a" (module))))
  (import "a" "m" (module (export "b" (module))))
  (import "a" "m" (module (export "b" (module (export "" (func))))))
  (import "a" "m" (module (export "c" (module))))
  (import "a" "m" (module (export "c" (module
    (export "a" (func))
  ))))
  (import "a" "m" (module (export "c" (module
    (export "a" (func))
    (export "b" (func (result i32)))
  ))))
  (import "a" "m" (module (export "c" (module
    (export "c" (global i32))
  ))))
  (import "a" "m" (module (export "c" (module
    (export "c" (global i32))
    (export "a" (func))
  ))))

  ;; for now import strings aren't matched at all, imports must simply pairwise
  ;; line up
  (import "a" "m" (module (export "d" (module (import "" (func))))))
  (import "a" "m" (module (export "d" (module (import "x" (func))))))
  (import "a" "m" (module (export "d" (module (import "x" "y" (func))))))

  (import "a" "m" (module (export "e" (module
    (import "x" "y" (func))
    (import "a" (func))
    (import "z" (global i32))
  ))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module (export "a" (func)))))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "d" (module)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "d" (module (import "" (module)))))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 2)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module (export "foo" (func)))))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "module types incompatible")

;; instances
(module $a
  ;; export nothing
  (module $m1)
  (instance (export "a") (instantiate $m1))
  ;; export one thing
  (module $m2
    (func (export ""))
  )
  (instance (export "b") (instantiate $m2))
  ;; export a mixture
  (module $m3
    (func (export "a"))
    (func (export "b") (result i32)
      i32.const 0)
    (global (export "c") i32 (i32.const 0))
  )
  (instance (export "c") (instantiate $m3))

  (module (export "m")
    ;; export one thing
    (module $m2
      (func (export ""))
    )
    (instance (export "i") (instantiate $m2))
  )

)
(module
  (import "a" "a" (instance))
  (import "a" "b" (instance))
  (import "a" "b" (instance (export "" (func))))
  (import "a" "c" (instance))
  (import "a" "c" (instance (export "a" (func))))
  (import "a" "c" (instance (export "b" (func (result i32)))))
  (import "a" "c" (instance (export "c" (global i32))))
  (import "a" "c" (instance
    (export "a" (func))
    (export "b" (func (result i32)))
    (export "c" (global i32))
  ))
  (import "a" "c" (instance
    (export "c" (global i32))
    (export "a" (func))
  ))

  (import "a" "m" (module (export "i" (instance))))
  (import "a" "m" (module (export "i" (instance (export "" (func))))))
)
(assert_unlinkable
  (module (import "a" "a" (instance (export "" (global f32)))))
  "instance types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "i" (instance (export "x" (func)))))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 2)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1 10)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (memory 2 10)))))
  "module types incompatible")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "module types incompatible")

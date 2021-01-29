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
)
(module
  (import "a" "m" (module (export "" (func))))
)
(module
  (import "a" "m" (module (export "a" (func))))
)
(module
  (import "a" "m" (module (export "b" (global i32))))
)
(module
  (import "a" "m" (module
    (export "" (func))
    (export "a" (func))
  ))
)
(module
  (import "a" "m" (module
    (export "a" (func))
    (export "" (func))
  ))
)
(module
  (import "a" "m" (module
    (export "a" (func))
    (export "" (func))
    (export "b" (global i32))
  ))
)
(module
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

(module (import "a" "m" (module)))
(module (import "a" "m" (module (export "" (func)))))
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func (param i32))))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func (result i32))))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global i32)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "incompatible import type for `a`")

(module $a
  (module (export "m")
    (global (export "") i32 (i32.const 0))))

;; globals
(module (import "a" "m" (module)))
(module (import "a" "m" (module (export "" (global i32)))))
(assert_unlinkable
  (module
    (import "a" "m" (module (export "" (global (mut i32)))))
  )
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "incompatible import type for `a`")

;; tables
(module $a
  (module (export "m")
    (table (export "") 1 funcref)
    (table (export "max") 1 10 funcref)
  )
)
(module
  (import "a" "m" (module))
)
(module
  (import "a" "m" (module (export "" (table 1 funcref))))
)
(module
  (import "a" "m" (module (export "" (table 0 funcref))))
)
(module
  (import "a" "m" (module (export "max" (table 1 10 funcref))))
)
(module
  (import "a" "m" (module (export "max" (table 0 10 funcref))))
)
(module
  (import "a" "m" (module (export "max" (table 0 11 funcref))))
)
(module
  (import "a" "m" (module (export "max" (table 0 funcref))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 2 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 10 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (table 2 10 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (table 1 9 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "incompatible import type for `a`")

;; memories
(module $a
  (module (export "m")
    (memory (export "") 1)
    (memory (export "max") 1 10)
  )
)
(module
  (import "a" "m" (module))
)
(module
  (import "a" "m" (module (export "" (memory 1))))
)
(module
  (import "a" "m" (module (export "" (memory 0))))
)
(module
  (import "a" "m" (module (export "max" (memory 1 10))))
)
(module
  (import "a" "m" (module (export "max" (memory 0 10))))
)
(module
  (import "a" "m" (module (export "max" (memory 0 11))))
)
(module
  (import "a" "m" (module (export "max" (memory 0))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 2)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1 10)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (memory 2 10)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (memory 2)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "incompatible import type for `a`")

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
      (import "a" (func))
      (import "b" (func))
      (import "c" (global i32))
    )
  )
)
(module
  (import "a" "m" (module))
)
(module
  (import "a" "m" (module (export "a" (module))))
)
(module
  (import "a" "m" (module (export "b" (module))))
)
(module
  (import "a" "m" (module (export "b" (module (export "" (func))))))
)
(module
  (import "a" "m" (module (export "c" (module))))
)
(module
  (import "a" "m" (module (export "c" (module
    (export "a" (func))
  ))))
)
(module
  (import "a" "m" (module (export "c" (module
    (export "a" (func))
    (export "b" (func (result i32)))
  ))))
)
(module
  (import "a" "m" (module (export "c" (module
    (export "c" (global i32))
  ))))
)
(module
  (import "a" "m" (module (export "c" (module
    (export "c" (global i32))
    (export "a" (func))
  ))))
)
(module
  (import "a" "m" (module (export "d" (module
    (import "" (func))
    (import "a" (func))
  ))))
)
(module
  (import "a" "m" (module (export "d" (module (import "" (func))))))
)
(assert_unlinkable
  (module
    (import "a" "m" (module (export "d" (module (import "x" (func))))))
  )
  "incompatible import type for `a`")
(assert_unlinkable
  (module
    (import "a" "m" (module (export "d" (module (import "x" "y" (func))))))
  )
  "incompatible import type for `a`")
(module
  (import "a" "m" (module (export "e" (module
    (import "a" (func))
    (import "b" (func))
    (import "c" (global i32))
  ))))
)
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module (export "a" (func)))))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "d" (module)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "d" (module (import "" (module)))))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (global f32)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 2)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module (export "foo" (func)))))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (instance)))))
  "incompatible import type for `a`")

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
)
(module
  (import "a" "b" (instance))
)
(module
  (import "a" "b" (instance (export "" (func))))
)
(module
  (import "a" "c" (instance))
)
(module
  (import "a" "c" (instance (export "a" (func))))
)
(module
  (import "a" "c" (instance (export "b" (func (result i32)))))
)
(module
  (import "a" "c" (instance (export "c" (global i32))))
)
(module
  (import "a" "c" (instance
    (export "a" (func))
    (export "b" (func (result i32)))
    (export "c" (global i32))
  ))
)
(module
  (import "a" "c" (instance
    (export "c" (global i32))
    (export "a" (func))
  ))
)
(module
  (import "a" "m" (module (export "i" (instance))))
)
(module
  (import "a" "m" (module (export "i" (instance (export "" (func))))))
)
(assert_unlinkable
  (module (import "a" "a" (instance (export "" (global f32)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "i" (instance (export "x" (func)))))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (func)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (table 1 funcref)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 2)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (memory 1 10)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "max" (memory 2 10)))))
  "incompatible import type for `a`")
(assert_unlinkable
  (module (import "a" "m" (module (export "" (module)))))
  "incompatible import type for `a`")

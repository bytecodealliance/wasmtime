;; simple nested component
(component
  (component)
)

;; simple nested component with a nested module
(component
  (component
    (core module)
  )
)

;; simple instantiation of a nested component
(component
  (component $c)
  (instance (instantiate $c))
  (instance (instantiate $c
    (with "x" (component $c))
  ))
)

;; instantiate a module during a nested component, and also instantiate it
;; as an export of the nested component
(component
  (component $c
    (core module $m)
    (core instance (instantiate $m))
    (export "m" (core module $m))
  )
  (instance $i (instantiate $c))
  (core instance $i (instantiate (module $i "m")))
)

;; instantiate an inner exported module with two different modules and
;; verify imports match
(component
  (component $c
    (core module $m
      (import "" "g" (global $g i32))
      (import "" "f" (func $f (result i32)))

      (func $start
        call $f
        global.get $g
        i32.ne
        if unreachable end)

      (start $start)
    )

    (core module $m2
      (global (export "g") i32 i32.const 1)
      (func (export "f") (result i32) i32.const 1)
    )
    (core instance $i2 (instantiate $m2))
    (core instance (instantiate $m (with "" (instance $i2))))

    (export "m" (core module $m))
  )
  (instance $i (instantiate $c))
  (core module $m2
    (global (export "g") i32 i32.const 5)
    (func (export "f") (result i32) i32.const 5)
  )
  (core instance $i2 (instantiate $m2))
  (core instance (instantiate (module $i "m") (with "" (instance $i2))))
)

;; instantiate an inner component with a module import
(component
  (component $c
    (import "m" (core module $m
      (export "g" (global i32))
    ))

    (core instance $i (instantiate $m))

    (core module $verify
      (import "" "g" (global $g i32))

      (func $start
        global.get $g
        i32.const 2
        i32.ne
        if unreachable end
      )

      (start $start)
    )
    (core instance (instantiate $verify (with "" (instance $i))))
  )

  (core module $m
    (global (export "g") i32 (i32.const 2))
  )
  (instance (instantiate $c (with "m" (core module $m))))
)

;; instantiate an inner component with a module import that itself has imports
(component
  (component $c
    (import "m" (core module $m
      (import "" "g" (global i32))
    ))
    (core module $m2
      (global (export "g") i32 i32.const 2100)
    )
    (core instance $m2 (instantiate $m2))
    (core instance (instantiate $m (with "" (instance $m2))))
  )

  (core module $verify
    (import "" "g" (global $g i32))

    (func $start
      global.get $g
      i32.const 2100
      i32.ne
      if unreachable end
    )

    (start $start)
  )
  (instance (instantiate $c (with "m" (core module $verify))))
)

;; instantiate an inner component with an export from the outer component
(component $c
  (core module (export "m")
    (import "" "g1" (global $g1 i32))
    (import "" "g2" (global $g2 i32))

    (func $start
      global.get $g1
      i32.const 10000
      i32.ne
      if unreachable end

      global.get $g2
      i32.const 20000
      i32.ne
      if unreachable end
    )

    (start $start)
  )
)

(component
  (import "c" (instance $i
    (export "m" (core module
      (import "" "g2" (global i32))
      (import "" "g1" (global i32))
    ))
  ))

  (component $c
    (import "m" (core module $verify
      (import "" "g2" (global i32))
      (import "" "g1" (global i32))
    ))

    (core module $m
      (global (export "g1") i32 i32.const 10000)
      (global (export "g2") i32 i32.const 20000)
    )
    (core instance $m (instantiate $m))
    (core instance (instantiate $verify (with "" (instance $m))))
  )

  (instance (instantiate $c (with "m" (core module $i "m"))))
)

;; instantiate a reexported module
(component
  (core module $m
    (global (export "g") i32 i32.const 7)
  )
  (component $c
    (import "i" (instance $i
      (export "m" (core module
        (import "" "" (func))
        (export "g" (global i32))
      ))
    ))

    (export "m" (core module $i "m"))
  )

  (instance $c (instantiate $c (with "i" (instance (export "m" (core module $m))))))
  (core module $dummy
    (func (export ""))
  )
  (core instance $dummy (instantiate $dummy))

  (core instance $m (instantiate (module $c "m") (with "" (instance $dummy))))

  (core module $verify
    (import "" "g" (global i32))
    (func $start
      global.get 0
      i32.const 7
      i32.ne
      if unreachable end
    )

    (start $start)
  )
  (core instance (instantiate $verify (with "" (instance $m))))
)

;; module must be found through a few layers of imports
(component $c
  (core module (export "m")
    (global (export "g") i32 i32.const 101)
  )
)

(component
  (import "c" (instance $i
    (export "m" (core module
      (export "g" (global i32))
    ))
  ))
  (component $c1
    (import "c" (instance $i
      (export "m" (core module
        (export "g" (global i32))
      ))
    ))
    (core module $verify
      (import "" "g" (global i32))
      (func $start
        global.get 0
        i32.const 101
        i32.ne
        if unreachable end
      )

      (start $start)
    )
    (core instance $m (instantiate (module $i "m")))
    (core instance (instantiate $verify (with "" (instance $m))))
  )
  (instance (instantiate $c1 (with "c" (instance $i))))
)

;; instantiate outer alias to self
(component $C
  (core module $m)
  (alias outer $C $m (core module $other_m))
  (core instance (instantiate $other_m))
)

(component $C
  (component $m)
  (alias outer $C $m (component $other_m))
  (instance (instantiate $other_m))
)


;; closing over an outer alias which is actually an argument to some
;; instantiation
(component
  (component $c
    (import "c" (core module $c
      (export "a" (global i32))
    ))

    (component (export "c")
      (export "m" (core module $c))
    )
  )

  (core module $m1 (global (export "a") i32 i32.const 1))
  (core module $m2 (global (export "a") i32 i32.const 2))

  (instance $c1 (instantiate $c (with "c" (core module $m1))))
  (instance $c2 (instantiate $c (with "c" (core module $m2))))

  (instance $m1_container (instantiate (component $c1 "c")))
  (instance $m2_container (instantiate (component $c2 "c")))

  (core instance $core1 (instantiate (module $m1_container "m")))
  (core instance $core2 (instantiate (module $m2_container "m")))

  (core module $verify
    (import "core1" "a" (global $a i32))
    (import "core2" "a" (global $b i32))

    (func $start
      global.get $a
      i32.const 1
      i32.ne
      if unreachable end

      global.get $b
      i32.const 2
      i32.ne
      if unreachable end
    )

    (start $start)
  )
  (core instance (instantiate $verify
    (with "core1" (instance $core1))
    (with "core2" (instance $core2))
  ))
)

;; simple importing of a component
(component
  (component $C)
  (component $other
    (import "x" (component $c))
    (instance (instantiate $c))
  )
  (instance (instantiate $other (with "x" (component $C))))
)

;; deep nesting
(component $C
  (core module $m
    (global (export "g") i32 (i32.const 1))
  )
  (component $c
    (core module (export "m")
      (global (export "g") i32 (i32.const 2))
    )
  )

  (component $c1
    (component $c2 (export "")
      (component $c3 (export "")
        (alias outer $C $m (core module $my_module))
        (alias outer $C $c (component $my_component))

        (export "m" (core module $my_module))
        (export "c" (component $my_component))
      )
    )
  )

  (instance $i1 (instantiate $c1))
  (instance $i2 (instantiate (component $i1 "")))
  (instance $i3 (instantiate (component $i2 "")))

  (core instance $m1 (instantiate (module $i3 "m")))
  (instance $c (instantiate (component $i3 "c")))
  (core instance $m2 (instantiate (module $c "m")))

  (core module $verify
    (import "m1" "g" (global $m1 i32))
    (import "m2" "g" (global $m2 i32))

    (func $start
      global.get $m1
      i32.const 1
      i32.ne
      if unreachable end

      global.get $m2
      i32.const 2
      i32.ne
      if unreachable end
    )
    (start $start)
  )
  (core instance (instantiate $verify (with "m1" (instance $m1)) (with "m2" (instance $m2))))
)

;; Try threading through component instantiation arguments as various forms of
;; instances.
(component
  (component $c
    (core module $m (export "m"))
    (component $c (export "c")
      (core module (export "m"))
    )
    (instance $i (instantiate $c))
    (instance $i2
      (export "m" (core module $m))
      (export "c" (component $c))
      (export "i" (instance $i))
    )
    (export "i" (instance $i))
    (export "i2" (instance $i2))
  )
  (instance $i (instantiate $c))

  (component $another
    (import "host" (instance
      (export "m" (core module))
      (export "c" (component))
      (export "i" (instance))
    ))
  )
  (instance (instantiate $another (with "host" (instance $i))))
  (instance (instantiate $another (with "host" (instance $i "i2"))))

  (instance $reexport
    (export "c" (component $i "c"))
    (export "m" (core module $i "m"))
    (export "i" (instance $i "i"))
  )
  (instance (instantiate $another (with "host" (instance $reexport))))
)

;; thread host functions around
(component
  (import "host-return-two" (func $import (result u32)))

  ;; thread the host function through an instance
  (component $c
    (import "" (func $f (result u32)))
    (export "f" (func $f))
  )
  (instance $c (instantiate $c (with "" (func $import))))
  (alias export $c "f" (func $import2))

  ;; thread the host function into a nested component
  (component $c2
    (import "host" (instance $i (export "return-two" (func (result u32)))))

    (core module $m
      (import "host" "return-two" (func $host (result i32)))
      (func $start
        call $host
        i32.const 2
        i32.ne
        if unreachable end
      )
      (start $start)
    )

    (core func $return_two
      (canon lower (func $i "return-two"))
    )
    (core instance (instantiate $m
      (with "host" (instance
        (export "return-two" (func $return_two))
      ))
    ))
  )

  (instance (instantiate $c2
    (with "host" (instance
      (export "return-two" (func $import2))
    ))
  ))
)

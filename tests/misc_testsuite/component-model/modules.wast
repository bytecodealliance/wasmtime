;;! reference_types = true

(component $foo
  (core module (export "a-module"))
)

;; the above instance can be imported into this component
(component
  (import "foo" (instance
    (export "a-module" (core module))
  ))
)

;; specifying extra imports is ok
(component
  (import "foo" (instance
    (export "a-module" (core module
      (import "foo" "bar" (func))
    ))
  ))
)

;; specifying extra exports is not ok
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "a-module" (core module
        (export "the-export" (func))
      ))
    ))
  )
  "module export `the-export` not defined")

(component $foo
  (core module (export "a-module")
    (import "env" "something" (func))
  )
)

;; imports must be specified
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "a-module" (core module))
    ))
  )
  "module import `env::something` not defined")

(component
  (import "foo" (instance
    (export "a-module" (core module
      (import "env" "something" (func))
    ))
  ))
)

;; extra imports still ok
(component
  (import "foo" (instance
    (export "a-module" (core module
      (import "env" "something" (func))
      (import "env" "other" (global i32))
    ))
  ))
)

(component $foo
  (core module (export "a-module")
    (func (export "f"))
  )
)

;; dropping exports is ok
(component
  (import "foo" (instance
    (export "a-module" (core module))
  ))
)

(component
  (import "foo" (instance
    (export "a-module" (core module
      (export "f" (func))
    ))
  ))
)

(assert_unlinkable
  (component
    (import "foo" (instance
      (export "a-module" (core module
        (export "f" (func (param i32)))
      ))
    ))
  )
  "expected type `(func (param i32))`, found type `(func)`")

(assert_unlinkable
  (component
    (import "foo" (instance
      (export "a-module" (core module
        (export "f" (global i32))
      ))
    ))
  )
  "expected global found func")

(component $foo
  (core module (export "m")
    (func (export "f"))
    (table (export "t") 1 funcref)
    (memory (export "m") 1)
    (global (export "g") i32 i32.const 0)
  )
)

;; wrong class of item
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "f" (global i32))))
    ))
  )
  "expected global found func")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "t" (func))))
    ))
  )
  "expected func found table")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "m" (func))))
    ))
  )
  "expected func found memory")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "g" (func))))
    ))
  )
  "expected func found global")

;; wrong item type
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "f" (func (param i32)))))
    ))
  )
  "export `f` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "t" (table 1 externref))))
    ))
  )
  "export `t` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "t" (table 2 funcref))))
    ))
  )
  "export `t` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "m" (memory 2))))
    ))
  )
  "export `m` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "g" (global f32))))
    ))
  )
  "export `g` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (export "g" (global (mut i32)))))
    ))
  )
  "export `g` has the wrong type")

;; subtyping ok
(component
  (import "foo" (instance
    (export "m" (core module
      (export "t" (table 0 funcref))
      (export "m" (memory 0))
    ))
  ))
)

(component $foo
  (core module (export "f") (func (import "" "")))
  (core module (export "t") (table (import "" "") 1 funcref))
  (core module (export "m") (memory (import "" "") 1))
  (core module (export "g") (global (import "" "") i32))
)

;; wrong class of item
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "f" (core module (import "" "" (global i32))))
    ))
  )
  "expected func found global")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "t" (core module (import "" "" (func))))
    ))
  )
  "expected table found func")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (import "" "" (func))))
    ))
  )
  "expected memory found func")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "g" (core module (import "" "" (func))))
    ))
  )
  "expected global found func")

;; wrong item type
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "f" (core module (import "" "" (func (param i32)))))
    ))
  )
  "module import `::` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "t" (core module (import "" "" (table 1 externref))))
    ))
  )
  "module import `::` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "t" (core module (import "" "" (table 0 funcref))))
    ))
  )
  "module import `::` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "m" (core module (import "" "" (memory 0))))
    ))
  )
  "module import `::` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "g" (core module (import "" "" (global f32))))
    ))
  )
  "module import `::` has the wrong type")
(assert_unlinkable
  (component
    (import "foo" (instance
      (export "g" (core module (import "" "" (global (mut i32)))))
    ))
  )
  "module import `::` has the wrong type")

;; subtyping ok, but in the opposite direction of imports
(component
  (import "foo" (instance
    (export "t" (core module (import "" "" (table 2 funcref))))
    (export "m" (core module (import "" "" (memory 2))))
  ))
)

;; An instance can reexport a module, define a module, and everything can be
;; used by something else
(component $src
  (core module (export "m")
    (global (export "g") i32 i32.const 2)
  )
)

(component $reexport
  (core module $m1
    (global (export "g") i32 i32.const 1)
  )
  (import "src" (instance $src
    (export "m" (core module (export "g" (global i32))))
  ))

  (core module $m3
    (global (export "g") i32 i32.const 3)
  )

  (export "m1" (core module $m1))
  (export "m2" (core module $src "m"))
  (export "m3" (core module $m3))
)

(component
  (core type $modulety (module (export "g" (global i32))))
  (import "reexport" (instance $reexport
    (export "m1" (core module (type $modulety)))
    (export "m2" (core module (type $modulety)))
    (export "m3" (core module (type $modulety)))
  ))

  (core module $assert_ok
    (import "m1" "g" (global $m1 i32))
    (import "m2" "g" (global $m2 i32))
    (import "m3" "g" (global $m3 i32))

    (func $assert_ok
      block
        global.get $m1
        i32.const 1
        i32.eq
        br_if 0
        unreachable
      end
      block
        global.get $m2
        i32.const 2
        i32.eq
        br_if 0
        unreachable
      end
      block
        global.get $m3
        i32.const 3
        i32.eq
        br_if 0
        unreachable
      end
    )

    (start $assert_ok)
  )

  (core instance $m1 (instantiate (module $reexport "m1")))
  (core instance $m2 (instantiate (module $reexport "m2")))
  (core instance $m3 (instantiate (module $reexport "m3")))

  (core instance (instantiate $assert_ok
    (with "m1" (instance $m1))
    (with "m2" (instance $m2))
    (with "m3" (instance $m3))
  ))
)

;; order of imports and exports can be shuffled between definition site and
;; use-site
(component $provider
  (core module (export "m")
    (import "" "1" (global $i1 i32))
    (import "" "2" (global $i2 i32))
    (import "" "3" (global $i3 i32))
    (import "" "4" (global $i4 i32))

    (global $g1 i32 i32.const 100)
    (global $g2 i32 i32.const 101)
    (global $g3 i32 i32.const 102)
    (global $g4 i32 i32.const 103)

    (func $assert_imports
      (block
        global.get $i1
        i32.const 1
        i32.eq
        br_if 0
        unreachable)
      (block
        global.get $i2
        i32.const 2
        i32.eq
        br_if 0
        unreachable)
      (block
        global.get $i3
        i32.const 3
        i32.eq
        br_if 0
        unreachable)
      (block
        global.get $i4
        i32.const 4
        i32.eq
        br_if 0
        unreachable)
    )

    (start $assert_imports)

    (export "g1" (global $g1))
    (export "g2" (global $g2))
    (export "g3" (global $g3))
    (export "g4" (global $g4))
  )
)

(component
  (import "provider" (instance $provider
    (export "m" (core module
      (import "" "4" (global i32))
      (import "" "3" (global i32))
      (import "" "2" (global i32))
      (import "" "1" (global i32))

      (export "g4" (global i32))
      (export "g3" (global i32))
      (export "g2" (global i32))
      (export "g1" (global i32))
    ))
  ))

  (core module $imports
    (global (export "1") i32 (i32.const 1))
    (global (export "3") i32 (i32.const 3))
    (global (export "2") i32 (i32.const 2))
    (global (export "4") i32 (i32.const 4))
  )
  (core instance $imports (instantiate $imports))
  (core instance $m (instantiate (module $provider "m")
    (with "" (instance $imports))
  ))

  (core module $import_globals
    (import "" "g4" (global $g4 i32))
    (import "" "g3" (global $g3 i32))
    (import "" "g2" (global $g2 i32))
    (import "" "g1" (global $g1 i32))

    (func $assert_imports
      (block
        global.get $g1
        i32.const 100
        i32.eq
        br_if 0
        unreachable)
      (block
        global.get $g2
        i32.const 101
        i32.eq
        br_if 0
        unreachable)
      (block
        global.get $g3
        i32.const 102
        i32.eq
        br_if 0
        unreachable)
      (block
        global.get $g4
        i32.const 103
        i32.eq
        br_if 0
        unreachable)
    )

    (start $assert_imports)
  )

  (core instance (instantiate $import_globals (with "" (instance $m))))
)

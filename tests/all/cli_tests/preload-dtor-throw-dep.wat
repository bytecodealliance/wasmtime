;; A `--preload` whose destructor throws an exception it does not catch. The
;; trampoline is a component boundary, so this must trap in the importer rather
;; than unwind into it.
(component
  (core module $d
    (type $t (func))
    (tag $t (type $t))
    (func (export "dtor") (param i32) throw $t)
  )
  (core instance $di (instantiate $d))

  (core module $m
    (func (export "make") (result i32) (i32.const 1))
  )
  (core instance $i (instantiate $m))

  (type $r (resource (rep i32) (dtor (core func $di "dtor"))))
  (core func $new (canon resource.new $r))

  (core module $w
    (import "" "make" (func $make (result i32)))
    (import "" "new" (func $new (param i32) (result i32)))
    (func (export "make") (result i32) (call $new (call $make)))
  )
  (core instance $wi (instantiate $w (with "" (instance
    (export "make" (func $i "make"))
    (export "new" (func $new))))))

  (func $make (result (own $r)) (canon lift (core func $wi "make")))
  (instance $inst
    (export "thing" (type $r))
    (export "make" (func $make)))
  (export "res" (instance $inst))
)

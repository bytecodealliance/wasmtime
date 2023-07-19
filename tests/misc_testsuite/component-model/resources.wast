;; bare bones "intrinsics work"
(component
  (type $r (resource (rep i32)))
  (core func $rep (canon resource.rep $r))
  (core func $new (canon resource.new $r))
  (core func $drop (canon resource.drop $r))

  (core module $m
     (import "" "rep" (func $rep (param i32) (result i32)))
     (import "" "new" (func $new (param i32) (result i32)))
     (import "" "drop" (func $drop (param i32)))

     (func $start
       (local $r i32)
       (local.set $r (call $new (i32.const 100)))

       (if (i32.ne (local.get $r) (i32.const 0)) (unreachable))
       (if (i32.ne (call $rep (local.get $r)) (i32.const 100)) (unreachable))

       (call $drop (local.get $r))
     )

     (start $start)
  )
  (core instance (instantiate $m
     (with "" (instance
       (export "rep" (func $rep))
       (export "new" (func $new))
       (export "drop" (func $drop))
     ))
  ))
)

;; cannot call `resource.drop` on a nonexistent resource
(component
  (type $r (resource (rep i32)))
  (core func $drop (canon resource.drop $r))

  (core module $m
     (import "" "drop" (func $drop (param i32)))

     (func (export "r")
       (call $drop (i32.const 0))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "drop" (func $drop))
     ))
  ))

  (func (export "r") (canon lift (core func $i "r")))
)
(assert_trap (invoke "r") "unknown handle index 0")

;; cannot call `resource.rep` on a nonexistent resource
(component
  (type $r (resource (rep i32)))
  (core func $rep (canon resource.rep $r))

  (core module $m
     (import "" "rep" (func $rep (param i32) (result i32)))

     (func (export "r")
       (drop (call $rep (i32.const 0)))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "rep" (func $rep))
     ))
  ))

  (func (export "r") (canon lift (core func $i "r")))
)
(assert_trap (invoke "r") "unknown handle index 0")

;; index reuse behavior of handles
(component
  (type $r (resource (rep i32)))
  (core func $rep (canon resource.rep $r))
  (core func $new (canon resource.new $r))
  (core func $drop (canon resource.drop $r))

  (core module $m
     (import "" "rep" (func $rep (param i32) (result i32)))
     (import "" "new" (func $new (param i32) (result i32)))
     (import "" "drop" (func $drop (param i32)))

     (func $start
       (local $r1 i32)
       (local $r2 i32)
       (local $r3 i32)
       (local $r4 i32)

       ;; resources assigned sequentially
       (local.set $r1 (call $new (i32.const 100)))
       (if (i32.ne (local.get $r1) (i32.const 0)) (unreachable))

       (local.set $r2 (call $new (i32.const 200)))
       (if (i32.ne (local.get $r2) (i32.const 1)) (unreachable))

       (local.set $r3 (call $new (i32.const 300)))
       (if (i32.ne (local.get $r3) (i32.const 2)) (unreachable))

       ;; representations all look good
       (if (i32.ne (call $rep (local.get $r1)) (i32.const 100)) (unreachable))
       (if (i32.ne (call $rep (local.get $r2)) (i32.const 200)) (unreachable))
       (if (i32.ne (call $rep (local.get $r3)) (i32.const 300)) (unreachable))

       ;; reallocate r2
       (call $drop (local.get $r2))
       (local.set $r2 (call $new (i32.const 400)))

       ;; should have reused index 1
       (if (i32.ne (local.get $r2) (i32.const 1)) (unreachable))

       ;; representations all look good
       (if (i32.ne (call $rep (local.get $r1)) (i32.const 100)) (unreachable))
       (if (i32.ne (call $rep (local.get $r2)) (i32.const 400)) (unreachable))
       (if (i32.ne (call $rep (local.get $r3)) (i32.const 300)) (unreachable))

       ;; deallocate, then reallocate
       (call $drop (local.get $r1))
       (call $drop (local.get $r2))
       (call $drop (local.get $r3))

       (local.set $r1 (call $new (i32.const 500)))
       (local.set $r2 (call $new (i32.const 600)))
       (local.set $r3 (call $new (i32.const 700)))

       ;; representations all look good
       (if (i32.ne (call $rep (local.get $r1)) (i32.const 500)) (unreachable))
       (if (i32.ne (call $rep (local.get $r2)) (i32.const 600)) (unreachable))
       (if (i32.ne (call $rep (local.get $r3)) (i32.const 700)) (unreachable))

       ;; indices should be lifo
       (if (i32.ne (local.get $r1) (i32.const 2)) (unreachable))
       (if (i32.ne (local.get $r2) (i32.const 1)) (unreachable))
       (if (i32.ne (local.get $r3) (i32.const 0)) (unreachable))

       ;; bump one more time
       (local.set $r4 (call $new (i32.const 800)))
       (if (i32.ne (local.get $r4) (i32.const 3)) (unreachable))

       ;; deallocate everything
       (call $drop (local.get $r1))
       (call $drop (local.get $r2))
       (call $drop (local.get $r3))
       (call $drop (local.get $r4))
     )

     (start $start)
  )
  (core instance (instantiate $m
     (with "" (instance
       (export "rep" (func $rep))
       (export "new" (func $new))
       (export "drop" (func $drop))
     ))
  ))
)

(assert_unlinkable
  (component
    (import "host" (instance
      (export "missing" (type (sub resource)))
    ))
  )
  "expected resource found nothing")
(assert_unlinkable
  (component
    (import "host" (instance
      (export "return-three" (type (sub resource)))
    ))
  )
  "expected resource found func")

;; all resources can be uniquely imported
(component
  (import "host" (instance
    (export "resource1" (type (sub resource)))
    (export "resource2" (type (sub resource)))
    (export "resource1-again" (type (sub resource)))
  ))
)

;; equality constraints also work
(component
  (import "host" (instance
    (export $r1 "resource1" (type (sub resource)))
    (export "resource2" (type (sub resource)))
    (export "resource1-again" (type (eq $r1)))
  ))
)

;; equality constraints are checked if resources are supplied
(assert_unlinkable
  (component
    (import "host" (instance
      (export "resource1" (type (sub resource)))
      (export $r1 "resource2" (type (sub resource)))
      (export "resource1-again" (type (eq $r1)))
    ))
  )
  "mismatched resource types")

;; equality constraints mean that types don't need to be supplied
(component
  (import "host" (instance
    (export $r1 "resource1" (type (sub resource)))
    (export "resource2" (type (sub resource)))
    (export "this-name-is-not-provided-in-the-wast-harness" (type (eq $r1)))
  ))
)

;; simple properties of handles
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[static]resource1.assert" (func (param "r" (own $r)) (param "rep" u32)))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[static]resource1.assert" (func $assert))

  (core func $drop (canon resource.drop $r))
  (core func $ctor (canon lower (func $ctor)))
  (core func $assert (canon lower (func $assert)))

  (core module $m
     (import "" "drop" (func $drop (param i32)))
     (import "" "ctor" (func $ctor (param i32) (result i32)))
     (import "" "assert" (func $assert (param i32 i32)))

     (func $start
       (local $r1 i32)
       (local $r2 i32)
       (local.set $r1 (call $ctor (i32.const 100)))
       (local.set $r2 (call $ctor (i32.const 200)))

       ;; assert r1/r2 are sequential
       (if (i32.ne (local.get $r1) (i32.const 0)) (unreachable))
       (if (i32.ne (local.get $r2) (i32.const 1)) (unreachable))

       ;; reallocate r1 and it should be reassigned the same index
       (call $drop (local.get $r1))
       (local.set $r1 (call $ctor (i32.const 300)))
       (if (i32.ne (local.get $r1) (i32.const 0)) (unreachable))

       ;; internal values should match
       (call $assert (local.get $r1) (i32.const 300))
       (call $assert (local.get $r2) (i32.const 200))
     )

     (start $start)
  )
  (core instance (instantiate $m
     (with "" (instance
       (export "drop" (func $drop))
       (export "ctor" (func $ctor))
       (export "assert" (func $assert))
     ))
  ))
)

;; Using an index that has never been valid is a trap
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[static]resource1.assert" (func (param "r" (own $r)) (param "rep" u32)))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[static]resource1.assert" (func $assert))
  (core func $assert (canon lower (func $assert)))

  (core module $m
     (import "" "assert" (func $assert (param i32 i32)))

     (func (export "f")
       (call $assert (i32.const 0) (i32.const 0))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "assert" (func $assert))
     ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "unknown handle index")

;; Using an index which was previously valid but no longer valid is also a trap.
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[static]resource1.assert" (func (param "r" (own $r)) (param "rep" u32)))
  ))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[static]resource1.assert" (func $assert))

  (core func $assert (canon lower (func $assert)))
  (core func $ctor (canon lower (func $ctor)))

  (core module $m
     (import "" "assert" (func $assert (param i32 i32)))
     (import "" "ctor" (func $ctor (param i32) (result i32)))

     (global $handle (mut i32) i32.const 0)

     (func (export "f")
        (global.set $handle (call $ctor (i32.const 100)))
        (call $assert (global.get $handle) (i32.const 100))
     )

     (func (export "f2")
        (call $assert (global.get $handle) (i32.const 100))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "assert" (func $assert))
       (export "ctor" (func $ctor))
     ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
  (func (export "f2") (canon lift (core func $i "f2")))
)

(assert_return (invoke "f"))
(assert_trap (invoke "f2") "unknown handle index")

;; Also invalid to pass a previously valid handle to the drop intrinsic
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))

  (core func $drop (canon resource.drop $r))
  (core func $ctor (canon lower (func $ctor)))

  (core module $m
     (import "" "drop" (func $drop (param i32)))
     (import "" "ctor" (func $ctor (param i32) (result i32)))

     (global $handle (mut i32) i32.const 0)

     (func (export "f")
        (global.set $handle (call $ctor (i32.const 100)))
        (call $drop (global.get $handle))
     )

     (func (export "f2")
        (call $drop (global.get $handle))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "ctor" (func $ctor))
       (export "drop" (func $drop))
     ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
  (func (export "f2") (canon lift (core func $i "f2")))
)

(assert_return (invoke "f"))
(assert_trap (invoke "f2") "unknown handle index")

;; If an inner component instantiates a resource then an outer component
;; should not implicitly have access to that resource.
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))

  ;; an inner component which upon instantiation will invoke the constructor,
  ;; assert that it's zero, and then forget about it.
  (component $inner
    (import "host" (instance $host
      (export $r "resource1" (type (sub resource)))
      (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    ))
    (alias export $host "[constructor]resource1" (func $ctor))

    (core func $ctor (canon lower (func $ctor)))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))

      (func $start
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 0)) (unreachable))
      )
    )
    (core instance $i (instantiate $m
       (with "" (instance (export "ctor" (func $ctor))))
    ))
  )
  (instance $i (instantiate $inner (with "host" (instance $host))))

  ;; the rest of this component which is a single function that invokes `drop`
  ;; for index 0. The index 0 should be valid within the above component, but
  ;; it is not valid within this component
  (alias export $host "resource1" (type $r))
  (core func $drop (canon resource.drop $r))

  (core module $m
     (import "" "drop" (func $drop (param i32)))

     (func (export "f")
        (call $drop (i32.const 0))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "drop" (func $drop))
     ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "unknown handle index")

;; Same as the above test, but for resources defined within a component
(component
  (component $inner
    (type $r (resource (rep i32)))

    (core func $ctor (canon resource.new $r))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))

      (func $start
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 0)) (unreachable))
      )
      (start $start)
    )
    (core instance $i (instantiate $m
       (with "" (instance (export "ctor" (func $ctor))))
    ))
    (export "r" (type $r))
  )
  (instance $i (instantiate $inner))

  ;; the rest of this component which is a single function that invokes `drop`
  ;; for index 0. The index 0 should be valid within the above component, but
  ;; it is not valid within this component
  (alias export $i "r" (type $r))
  (core func $drop (canon resource.drop $r))

  (core module $m
     (import "" "drop" (func $drop (param i32)))

     (func (export "f")
        (call $drop (i32.const 0))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "drop" (func $drop))
     ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "unknown handle index")

;; Each instantiation of a component generates a unique resource type, so
;; allocating in one component and deallocating in another should fail.
(component
  (component $inner
    (type $r (resource (rep i32)))

    (core func $ctor (canon resource.new $r))
    (core func $drop (canon resource.drop $r))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))

      (func (export "alloc")
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 0)) (unreachable))
      )
      (func (export "dealloc")
        (call $drop (i32.const 0))
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "ctor" (func $ctor))
        (export "drop" (func $drop))
      ))
    ))
    (func (export "alloc") (canon lift (core func $i "alloc")))
    (func (export "dealloc") (canon lift (core func $i "dealloc")))
  )
  (instance $i1 (instantiate $inner))
  (instance $i2 (instantiate $inner))

  (alias export $i1 "alloc" (func $alloc_in_1))
  (alias export $i1 "dealloc" (func $dealloc_in_1))
  (alias export $i2 "alloc" (func $alloc_in_2))
  (alias export $i2 "dealloc" (func $dealloc_in_2))

  (export "alloc-in1" (func $alloc_in_1))
  (export "dealloc-in1" (func $dealloc_in_1))
  (export "alloc-in2" (func $alloc_in_2))
  (export "dealloc-in2" (func $dealloc_in_2))
)

(assert_return (invoke "alloc-in1"))
(assert_return (invoke "dealloc-in1"))
(assert_return (invoke "alloc-in1"))
(assert_return (invoke "alloc-in2"))
(assert_return (invoke "dealloc-in2"))
(assert_trap (invoke "dealloc-in2") "unknown handle index")

;; Same as above, but the same host resource type is imported into a
;; component that is instantiated twice. Each component instance should
;; receive different tables tracking resources so a resource allocated in one
;; should not be visible in the other.
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))

  (component $inner
    (import "r" (type $r (sub resource)))
    (import "[constructor]r" (func $ctor (param "r" u32) (result (own $r))))

    (core func $ctor (canon lower (func $ctor)))
    (core func $drop (canon resource.drop $r))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))

      (func (export "alloc")
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 0)) (unreachable))
      )
      (func (export "dealloc")
        (call $drop (i32.const 0))
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "ctor" (func $ctor))
        (export "drop" (func $drop))
      ))
    ))
    (func (export "alloc") (canon lift (core func $i "alloc")))
    (func (export "dealloc") (canon lift (core func $i "dealloc")))
  )
  (instance $i1 (instantiate $inner
    (with "r" (type $r))
    (with "[constructor]r" (func $ctor))
  ))
  (instance $i2 (instantiate $inner
    (with "r" (type $r))
    (with "[constructor]r" (func $ctor))
  ))

  (alias export $i1 "alloc" (func $alloc_in_1))
  (alias export $i1 "dealloc" (func $dealloc_in_1))
  (alias export $i2 "alloc" (func $alloc_in_2))
  (alias export $i2 "dealloc" (func $dealloc_in_2))

  (export "alloc-in1" (func $alloc_in_1))
  (export "dealloc-in1" (func $dealloc_in_1))
  (export "alloc-in2" (func $alloc_in_2))
  (export "dealloc-in2" (func $dealloc_in_2))
)

(assert_return (invoke "alloc-in1"))
(assert_return (invoke "dealloc-in1"))
(assert_return (invoke "alloc-in1"))
(assert_return (invoke "alloc-in2"))
(assert_return (invoke "dealloc-in2"))
(assert_trap (invoke "dealloc-in2") "unknown handle index")

;; Multiple copies of intrinsics all work
(component
  (type $r (resource (rep i32)))

  (core func $new1 (canon resource.new $r))
  (core func $new2 (canon resource.new $r))
  (core func $drop1 (canon resource.drop $r))
  (core func $drop2 (canon resource.drop $r))

  (core module $m
    (import "" "new1" (func $new1 (param i32) (result i32)))
    (import "" "new2" (func $new2 (param i32) (result i32)))
    (import "" "drop1" (func $drop1 (param i32)))
    (import "" "drop2" (func $drop2 (param i32)))

    (func $start
      ;; 2x2 matrix of pairing new/drop
      (call $drop1 (call $new1 (i32.const 101)))
      (call $drop2 (call $new1 (i32.const 102)))
      (call $drop1 (call $new2 (i32.const 103)))
      (call $drop2 (call $new2 (i32.const 104)))

      ;; should be referencing the same namespace
      (if (i32.ne (call $new1 (i32.const 105)) (i32.const 0)) (unreachable))
      (if (i32.ne (call $new2 (i32.const 105)) (i32.const 1)) (unreachable))

      ;; use different drops out of order
      (call $drop2 (i32.const 0))
      (call $drop1 (i32.const 1))
    )

    (start $start)
  )

  (core instance (instantiate $m
    (with "" (instance
      (export "new1" (func $new1))
      (export "new2" (func $new2))
      (export "drop1" (func $drop1))
      (export "drop2" (func $drop2))
    ))
  ))
)

;; u32::MAX isn't special in some weird way, it's just probably always invalid
;; because that's a lot of handles.
(component
  (type $r (resource (rep i32)))

  (core func $drop (canon resource.drop $r))

  (core module $m
    (import "" "drop" (func $drop (param i32)))

    (func (export "f")
      (call $drop (i32.const 0xffffffff))
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "drop" (func $drop))
    ))
  ))
  (func (export "f") (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "unknown handle index")

;; Test behavior of running a destructor for local resources
(component
  (core module $m1
    (global $drops (mut i32) i32.const 0)
    (global $last_drop (mut i32) i32.const -1)

    (func (export "dtor") (param i32)
      (global.set $drops (i32.add (global.get $drops) (i32.const 1)))
      (global.set $last_drop (local.get 0))
    )
    (func (export "drops") (result i32) global.get $drops)
    (func (export "last-drop") (result i32) global.get $last_drop)
  )
  (core instance $i1 (instantiate $m1))

  (type $r1 (resource (rep i32)))
  (type $r2 (resource (rep i32) (dtor (func $i1 "dtor"))))

  (core func $drop1 (canon resource.drop $r1))
  (core func $drop2 (canon resource.drop $r2))
  (core func $new1 (canon resource.new $r1))
  (core func $new2 (canon resource.new $r2))

  (core module $m2
    (import "" "drop1" (func $drop1 (param i32)))
    (import "" "drop2" (func $drop2 (param i32)))
    (import "" "new1" (func $new1 (param i32) (result i32)))
    (import "" "new2" (func $new2 (param i32) (result i32)))
    (import "i1" "drops" (func $drops (result i32)))
    (import "i1" "last-drop" (func $last-drop (result i32)))

    (func $start
      (local $r1 i32)
      (local $r2 i32)

      (local.set $r1 (call $new1 (i32.const 100)))
      (local.set $r2 (call $new2 (i32.const 200)))

      ;; both should be index 0
      (if (i32.ne (local.get $r1) (i32.const 0)) (unreachable))
      (if (i32.ne (local.get $r2) (i32.const 0)) (unreachable))

      ;; nothing should be dropped yet
      (if (i32.ne (call $drops) (i32.const 0)) (unreachable))
      (if (i32.ne (call $last-drop) (i32.const -1)) (unreachable))

      ;; dropping a resource without a destructor is ok, but shouldn't tamper
      ;; with anything.
      (call $drop1 (local.get $r1))
      (if (i32.ne (call $drops) (i32.const 0)) (unreachable))
      (if (i32.ne (call $last-drop) (i32.const -1)) (unreachable))

      ;; drop r2 which should record a drop and additionally record the private
      ;; representation value which was dropped
      (call $drop2 (local.get $r2))
      (if (i32.ne (call $drops) (i32.const 1)) (unreachable))
      (if (i32.ne (call $last-drop) (i32.const 200)) (unreachable))

      ;; do it all over again
      (local.set $r2 (call $new2 (i32.const 300)))
      (call $drop2 (local.get $r2))
      (if (i32.ne (call $drops) (i32.const 2)) (unreachable))
      (if (i32.ne (call $last-drop) (i32.const 300)) (unreachable))
    )

    (start $start)
  )

  (core instance $i2 (instantiate $m2
    (with "" (instance
      (export "drop1" (func $drop1))
      (export "drop2" (func $drop2))
      (export "new1" (func $new1))
      (export "new2" (func $new2))
    ))
    (with "i1" (instance $i1))
  ))
)

;; Test dropping a host resource
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[static]resource1.last-drop" (func (result u32)))
    (export "[static]resource1.drops" (func (result u32)))
  ))

  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[static]resource1.last-drop" (func $last-drop))
  (alias export $host "[static]resource1.drops" (func $drops))

  (core func $drop (canon resource.drop $r))
  (core func $ctor (canon lower (func $ctor)))
  (core func $last-drop (canon lower (func $last-drop)))
  (core func $drops (canon lower (func $drops)))

  (core module $m
    (import "" "drop" (func $drop (param i32)))
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "last-drop" (func $last-drop (result i32)))
    (import "" "drops" (func $raw-drops (result i32)))

    (global $init-drop-cnt (mut i32) i32.const 0)

    (func $drops (result i32)
      (i32.sub (call $raw-drops) (global.get $init-drop-cnt))
    )

    (func $start
      (local $r1 i32)
      (global.set $init-drop-cnt (call $raw-drops))

      (local.set $r1 (call $ctor (i32.const 100)))

      ;; should be no drops yet
      (if (i32.ne (call $drops) (i32.const 0)) (unreachable))

      ;; should count a drop
      (call $drop (local.get $r1))
      (if (i32.ne (call $drops) (i32.const 1)) (unreachable))
      (if (i32.ne (call $last-drop) (i32.const 100)) (unreachable))

      ;; do it again to be sure
      (local.set $r1 (call $ctor (i32.const 200)))
      (call $drop (local.get $r1))
      (if (i32.ne (call $drops) (i32.const 2)) (unreachable))
      (if (i32.ne (call $last-drop) (i32.const 200)) (unreachable))
    )

    (start $start)
  )
  (core instance (instantiate $m
    (with "" (instance
      (export "drop" (func $drop))
      (export "ctor" (func $ctor))
      (export "last-drop" (func $last-drop))
      (export "drops" (func $drops))
    ))
  ))
)

;; Test some bare-bones basics of borrowed resources
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[method]resource1.simple" (func (param "self" (borrow $r)) (param "rep" u32)))
    (export "[method]resource1.take-borrow" (func (param "self" (borrow $r)) (param "b" (borrow $r))))
    (export "[method]resource1.take-own" (func (param "self" (borrow $r)) (param "b" (own $r))))
  ))

  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[method]resource1.simple" (func $simple))
  (alias export $host "[method]resource1.take-borrow" (func $take-borrow))
  (alias export $host "[method]resource1.take-own" (func $take-own))

  (core func $drop (canon resource.drop $r))
  (core func $ctor (canon lower (func $ctor)))
  (core func $simple (canon lower (func $simple)))
  (core func $take-own (canon lower (func $take-own)))
  (core func $take-borrow (canon lower (func $take-borrow)))

  (core module $m
    (import "" "drop" (func $drop (param i32)))
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "simple" (func $simple (param i32 i32)))
    (import "" "take-own" (func $take-own (param i32 i32)))
    (import "" "take-borrow" (func $take-borrow (param i32 i32)))


    (func $start
      (local $r1 i32)
      (local $r2 i32)
      (local.set $r1 (call $ctor (i32.const 100)))
      (local.set $r2 (call $ctor (i32.const 200)))

      (call $simple (local.get $r1) (i32.const 100))
      (call $simple (local.get $r1) (i32.const 100))
      (call $simple (local.get $r2) (i32.const 200))
      (call $simple (local.get $r1) (i32.const 100))
      (call $simple (local.get $r2) (i32.const 200))
      (call $simple (local.get $r2) (i32.const 200))

      (call $drop (local.get $r1))
      (call $drop (local.get $r2))


      (local.set $r1 (call $ctor (i32.const 200)))
      (local.set $r2 (call $ctor (i32.const 300)))
      (call $take-borrow (local.get $r1) (local.get $r2))
      (call $take-borrow (local.get $r2) (local.get $r1))
      (call $take-borrow (local.get $r1) (local.get $r1))
      (call $take-borrow (local.get $r2) (local.get $r2))

      (call $take-own (local.get $r1) (call $ctor (i32.const 400)))
      (call $take-own (local.get $r2) (call $ctor (i32.const 500)))
      (call $take-own (local.get $r2) (local.get $r1))
      (call $drop (local.get $r2))

      ;; table should be empty at this point, so a fresh allocation should get
      ;; index 0
      (if (i32.ne (call $ctor (i32.const 600)) (i32.const 0)) (unreachable))
    )

    (start $start)
  )
  (core instance (instantiate $m
    (with "" (instance
      (export "drop" (func $drop))
      (export "ctor" (func $ctor))
      (export "simple" (func $simple))
      (export "take-own" (func $take-own))
      (export "take-borrow" (func $take-borrow))
    ))
  ))
)

;; Cannot pass out an owned resource when it's borrowed by the same call
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[method]resource1.take-own" (func (param "self" (borrow $r)) (param "b" (own $r))))
  ))

  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[method]resource1.take-own" (func $take-own))

  (core func $drop (canon resource.drop $r))
  (core func $ctor (canon lower (func $ctor)))
  (core func $take-own (canon lower (func $take-own)))

  (core module $m
    (import "" "drop" (func $drop (param i32)))
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "take-own" (func $take-own (param i32 i32)))


    (func (export "f")
      (local $r i32)
      (local.set $r (call $ctor (i32.const 100)))
      (call $take-own (local.get $r) (local.get $r))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "drop" (func $drop))
      (export "ctor" (func $ctor))
      (export "take-own" (func $take-own))
    ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "cannot remove owned resource while borrowed")

;; Borrows must actually exist
(component
  (import "host" (instance $host
    (export $r "resource1" (type (sub resource)))
    (export "[method]resource1.simple" (func (param "self" (borrow $r)) (param "b" u32)))
  ))

  (alias export $host "resource1" (type $r))
  (alias export $host "[method]resource1.simple" (func $simple))

  (core func $drop (canon resource.drop $r))
  (core func $simple (canon lower (func $simple)))

  (core module $m
    (import "" "drop" (func $drop (param i32)))
    (import "" "simple" (func $simple (param i32 i32)))


    (func (export "f")
      (call $simple (i32.const 0) (i32.const 0))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "drop" (func $drop))
      (export "simple" (func $simple))
    ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "unknown handle index 0")

(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))
    (core func $dtor (canon resource.drop $t))
    (core func $rep (canon resource.rep $t))

    (core module $m
      (import "" "dtor" (func $dtor (param i32)))
      (import "" "rep" (func $rep (param i32) (result i32)))

      (func (export "[method]t.assert") (param i32 i32)
        (if (i32.ne (local.get 0) (local.get 1)) (unreachable))
      )
      (func (export "[static]t.assert-own") (param i32 i32)
        (if (i32.ne (call $rep (local.get 0)) (local.get 1)) (unreachable))
        (call $dtor (local.get 0))
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "dtor" (func $dtor))
        (export "rep" (func $rep))
      ))
    ))
    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor)))
    (func (export "[method]t.assert") (param "self" (borrow $t)) (param "x" u32)
      (canon lift (core func $i "[method]t.assert")))
    (func (export "[static]t.assert-own") (param "self" (own $t)) (param "x" u32)
      (canon lift (core func $i "[static]t.assert-own")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export $t "t" (type (sub resource)))
      (export "[constructor]t" (func (param "x" u32) (result (own $t))))
      (export "[method]t.assert" (func (param "self" (borrow $t)) (param "x" u32)))
      (export "[static]t.assert-own" (func (param "self" (own $t)) (param "x" u32)))
    ))

    (alias export $i "t" (type $t))
    (alias export $i "[constructor]t" (func $ctor))
    (alias export $i "[method]t.assert" (func $assert-borrow))
    (alias export $i "[static]t.assert-own" (func $assert-own))

    (core func $ctor (canon lower (func $ctor)))
    (core func $dtor (canon resource.drop $t))
    (core func $assert-own (canon lower (func $assert-own)))
    (core func $assert-borrow (canon lower (func $assert-borrow)))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "dtor" (func $dtor (param i32)))
      (import "" "assert-own" (func $assert-own (param i32 i32)))
      (import "" "assert-borrow" (func $assert-borrow (param i32 i32)))

      (func (export "f")
        (local $r1 i32)
        (local $r2 i32)

        (local.set $r1 (call $ctor (i32.const 100)))
        (local.set $r2 (call $ctor (i32.const 200)))

        (if (i32.ne (local.get $r1) (i32.const 0)) (unreachable))
        (if (i32.ne (local.get $r2) (i32.const 1)) (unreachable))

        (call $assert-borrow (local.get $r2) (i32.const 200))
        (call $assert-borrow (local.get $r1) (i32.const 100))

        (call $assert-own (local.get $r2) (i32.const 200))
        (call $dtor (local.get $r1))
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "ctor" (func $ctor))
        (export "dtor" (func $dtor))
        (export "assert-own" (func $assert-own))
        (export "assert-borrow" (func $assert-borrow))
      ))
    ))
    (func (export "f") (canon lift (core func $i "f")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "f" (func $b "f"))
)

(assert_return (invoke "f"))

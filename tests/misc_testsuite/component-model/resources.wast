;;! component_model_async = true

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

       (if (i32.ne (local.get $r) (i32.const 1)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r)) (i32.const 100)) (then (unreachable)))

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
       (if (i32.ne (local.get $r1) (i32.const 1)) (then (unreachable)))

       (local.set $r2 (call $new (i32.const 200)))
       (if (i32.ne (local.get $r2) (i32.const 2)) (then (unreachable)))

       (local.set $r3 (call $new (i32.const 300)))
       (if (i32.ne (local.get $r3) (i32.const 3)) (then (unreachable)))

       ;; representations all look good
       (if (i32.ne (call $rep (local.get $r1)) (i32.const 100)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r2)) (i32.const 200)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r3)) (i32.const 300)) (then (unreachable)))

       ;; reallocate r2
       (call $drop (local.get $r2))
       (local.set $r2 (call $new (i32.const 400)))

       ;; should have reused index 3
       (if (i32.ne (local.get $r2) (i32.const 2)) (then (unreachable)))

       ;; representations all look good
       (if (i32.ne (call $rep (local.get $r1)) (i32.const 100)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r2)) (i32.const 400)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r3)) (i32.const 300)) (then (unreachable)))

       ;; deallocate, then reallocate
       (call $drop (local.get $r1))
       (call $drop (local.get $r2))
       (call $drop (local.get $r3))

       (local.set $r1 (call $new (i32.const 500)))
       (local.set $r2 (call $new (i32.const 600)))
       (local.set $r3 (call $new (i32.const 700)))

       ;; representations all look good
       (if (i32.ne (call $rep (local.get $r1)) (i32.const 500)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r2)) (i32.const 600)) (then (unreachable)))
       (if (i32.ne (call $rep (local.get $r3)) (i32.const 700)) (then (unreachable)))

       ;; indices should be lifo
       (if (i32.ne (local.get $r1) (i32.const 3)) (then (unreachable)))
       (if (i32.ne (local.get $r2) (i32.const 2)) (then (unreachable)))
       (if (i32.ne (local.get $r3) (i32.const 1)) (then (unreachable)))

       ;; bump one more time
       (local.set $r4 (call $new (i32.const 800)))
       (if (i32.ne (local.get $r4) (i32.const 4)) (then (unreachable)))

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
  "was not found")
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
    (export "resource1" (type $r1 (sub resource)))
    (export "resource2" (type (sub resource)))
    (export "resource1-again" (type (eq $r1)))
  ))
)

;; equality constraints are checked if resources are supplied
(assert_unlinkable
  (component
    (import "host" (instance
      (export "resource1" (type (sub resource)))
      (export "resource2" (type $r1 (sub resource)))
      (export "resource1-again" (type (eq $r1)))
    ))
  )
  "mismatched resource types")

;; equality constraints mean that types don't need to be supplied
(component
  (import "host" (instance
    (export "resource1" (type $r1 (sub resource)))
    (export "resource2" (type (sub resource)))
    (export "this-name-is-not-provided-in-the-wast-harness" (type (eq $r1)))
  ))
)

;; simple properties of handles
(component
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
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
       (if (i32.ne (local.get $r1) (i32.const 1)) (then (unreachable)))
       (if (i32.ne (local.get $r2) (i32.const 2)) (then (unreachable)))

       ;; reallocate r1 and it should be reassigned the same index
       (call $drop (local.get $r1))
       (local.set $r1 (call $ctor (i32.const 300)))
       (if (i32.ne (local.get $r1) (i32.const 1)) (then (unreachable)))

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
    (export "resource1" (type $r (sub resource)))
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
    (export "resource1" (type $r (sub resource)))
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
    (export "resource1" (type $r (sub resource)))
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
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))

  ;; an inner component which upon instantiation will invoke the constructor,
  ;; assert that it's zero, and then forget about it.
  (component $inner
    (import "host" (instance $host
      (export "resource1" (type $r (sub resource)))
      (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    ))
    (alias export $host "[constructor]resource1" (func $ctor))

    (core func $ctor (canon lower (func $ctor)))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))

      (func $start
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 0)) (then (unreachable)))
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
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 1)) (then (unreachable)))
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
  ;; for index 2. The index 2 should be valid within the above component, but
  ;; it is not valid within this component
  (alias export $i "r" (type $r))
  (core func $drop (canon resource.drop $r))

  (core module $m
     (import "" "drop" (func $drop (param i32)))

     (func (export "f")
        (call $drop (i32.const 2))
     )
  )
  (core instance $i (instantiate $m
     (with "" (instance
       (export "drop" (func $drop))
     ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "unknown handle index 2")

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
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 1)) (then (unreachable)))
      )
      (func (export "dealloc")
        (call $drop (i32.const 1))
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
    (export "resource1" (type $r (sub resource)))
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
        (if (i32.ne (call $ctor (i32.const 100)) (i32.const 1)) (then (unreachable)))
      )
      (func (export "dealloc")
        (call $drop (i32.const 1))
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
      (if (i32.ne (call $new1 (i32.const 105)) (i32.const 1)) (then (unreachable)))
      (if (i32.ne (call $new2 (i32.const 105)) (i32.const 2)) (then (unreachable)))

      ;; use different drops out of order
      (call $drop2 (i32.const 1))
      (call $drop1 (i32.const 2))
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
  (type $r2 (resource (rep i32) (dtor (core func $i1 "dtor"))))

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

      ;; indexes start at 1 and while they have distinct types they should be
      ;; within the same table.
      (if (i32.ne (local.get $r1) (i32.const 1)) (then (unreachable)))
      (if (i32.ne (local.get $r2) (i32.const 2)) (then (unreachable)))

      ;; nothing should be dropped yet
      (if (i32.ne (call $drops) (i32.const 0)) (then (unreachable)))
      (if (i32.ne (call $last-drop) (i32.const -1)) (then (unreachable)))

      ;; dropping a resource without a destructor is ok, but shouldn't tamper
      ;; with anything.
      (call $drop1 (local.get $r1))
      (if (i32.ne (call $drops) (i32.const 0)) (then (unreachable)))
      (if (i32.ne (call $last-drop) (i32.const -1)) (then (unreachable)))

      ;; drop r2 which should record a drop and additionally record the private
      ;; representation value which was dropped
      (call $drop2 (local.get $r2))
      (if (i32.ne (call $drops) (i32.const 1)) (then (unreachable)))
      (if (i32.ne (call $last-drop) (i32.const 200)) (then (unreachable)))

      ;; do it all over again
      (local.set $r2 (call $new2 (i32.const 300)))
      (call $drop2 (local.get $r2))
      (if (i32.ne (call $drops) (i32.const 2)) (then (unreachable)))
      (if (i32.ne (call $last-drop) (i32.const 300)) (then (unreachable)))
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
    (export "resource1" (type $r (sub resource)))
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
      (if (i32.ne (call $drops) (i32.const 0)) (then (unreachable)))

      ;; should count a drop
      (call $drop (local.get $r1))
      (if (i32.ne (call $drops) (i32.const 1)) (then (unreachable)))
      (if (i32.ne (call $last-drop) (i32.const 100)) (then (unreachable)))

      ;; do it again to be sure
      (local.set $r1 (call $ctor (i32.const 200)))
      (call $drop (local.get $r1))
      (if (i32.ne (call $drops) (i32.const 2)) (then (unreachable)))
      (if (i32.ne (call $last-drop) (i32.const 200)) (then (unreachable)))
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
    (export "resource1" (type $r (sub resource)))
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
      ;; index 2
      (if (i32.ne (call $ctor (i32.const 600)) (i32.const 1)) (then (unreachable)))
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
    (export "resource1" (type $r (sub resource)))
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
    (export "resource1" (type $r (sub resource)))
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
        (if (i32.ne (local.get 0) (local.get 1)) (then (unreachable)))
      )
      (func (export "[static]t.assert-own") (param i32 i32)
        (if (i32.ne (call $rep (local.get 0)) (local.get 1)) (then (unreachable)))
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
      (export "t" (type $t (sub resource)))
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

        (if (i32.ne (local.get $r1) (i32.const 1)) (then (unreachable)))
        (if (i32.ne (local.get $r2) (i32.const 2)) (then (unreachable)))

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

;; Test destructor behavior when using the wrong resource type
(component definition $C
  (type $r1 (resource (rep i32)))
  (type $r2 (resource (rep i32)))

  (core func $drop1 (canon resource.drop $r1))
  (core func $drop2 (canon resource.drop $r2))
  (core func $new1 (canon resource.new $r1))
  (core func $new2 (canon resource.new $r2))

  (core module $m2
    (import "" "drop1" (func $drop1 (param i32)))
    (import "" "drop2" (func $drop2 (param i32)))
    (import "" "new1" (func $new1 (param i32) (result i32)))
    (import "" "new2" (func $new2 (param i32) (result i32)))

    (func (export "drop-r1-as-r2") (call $drop2 (call $new1 (i32.const 100))))
    (func (export "return-r1-as-r2") (result i32) (call $new1 (i32.const 100)))
  )

  (core instance $i2 (instantiate $m2
    (with "" (instance
      (export "drop1" (func $drop1))
      (export "drop2" (func $drop2))
      (export "new1" (func $new1))
      (export "new2" (func $new2))
    ))
  ))

  (export $r2' "r2" (type $r2))
  (func (export "drop-r1-as-r2") (canon lift (core func $i2 "drop-r1-as-r2")))
  (func (export "return-r1-as-r2") (result (own $r2')) (canon lift (core func $i2 "return-r1-as-r2")))
)

(component instance $C1 $C)
(assert_trap (invoke "drop-r1-as-r2") "handle index 1 used with the wrong type, expected guest-defined resource but found a different guest-defined resource")
(component instance $C1 $C)
(assert_trap (invoke "return-r1-as-r2") "handle index 1 used with the wrong type, expected guest-defined resource but found a different guest-defined resource")

;; Test that `resource.rep` is exempt from may-leave checks
(component
  (type $r (resource (rep i32)))
  (core func $resource.new (canon resource.new $r))
  (core func $resource.rep (canon resource.rep $r))

  (core module $DM
    (import "" "resource.new" (func $resource.new (param i32) (result i32)))
    (import "" "resource.rep" (func $resource.rep (param i32) (result i32)))

    (global $g (mut i32) (i32.const 0))

    (func (export "run")
      (global.set $g (call $resource.new (i32.const 42)))
    )
    (func (export "post-return")
      (i32.eq
        (call $resource.rep (global.get $g))
        (i32.const 42))
      if return end
      unreachable
    )
  )
  (core instance $dm (instantiate $DM (with "" (instance
    (export "resource.new" (func $resource.new))
    (export "resource.rep" (func $resource.rep))
  ))))
  (func (export "run")
    (canon lift (core func $dm "run") (post-return (core func $dm "post-return"))))
)

(assert_return (invoke "run"))

;; A resource defined by the outer component can be dropped by the outer
;; component and by two separate nested instances.
(component
  (type $t' (resource (rep i32)))
  (export $t "t" (type $t'))

  (core func $ctor (canon resource.new $t))
  (func $ctor (param "x" u32) (result (own $t))
    (canon lift (core func $ctor)))
  (core func $dtor (canon resource.drop $t))
  (func $dtor1 (param "x" (own $t)) (canon lift (core func $dtor)))

  (component $C
    (import "t" (type $t (sub resource)))
    (core func $dtor (canon resource.drop $t))
    (func (export "dtor") (param "x" (own $t))
      (canon lift (core func $dtor)))
  )
  (instance $i1 (instantiate $C (with "t" (type $t))))
  (instance $i2 (instantiate $C (with "t" (type $t))))

  (component $Driver
    (import "t" (type $t (sub resource)))
    (import "ctor" (func $ctor (param "x" u32) (result (own $t))))
    (import "dtor1" (func $dtor1 (param "x" (own $t))))
    (import "dtor2" (func $dtor2 (param "x" (own $t))))
    (import "dtor3" (func $dtor3 (param "x" (own $t))))

    (core func $ctor (canon lower (func $ctor)))
    (core func $dtor1 (canon lower (func $dtor1)))
    (core func $dtor2 (canon lower (func $dtor2)))
    (core func $dtor3 (canon lower (func $dtor3)))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "dtor1" (func $dtor1 (param i32)))
      (import "" "dtor2" (func $dtor2 (param i32)))
      (import "" "dtor3" (func $dtor3 (param i32)))
      (func (export "run")
        (call $dtor1 (call $ctor (i32.const 100)))
        (call $dtor2 (call $ctor (i32.const 200)))
        (call $dtor3 (call $ctor (i32.const 300))))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "ctor" (func $ctor))
        (export "dtor1" (func $dtor1))
        (export "dtor2" (func $dtor2))
        (export "dtor3" (func $dtor3))
      ))
    ))
    (func (export "run") (canon lift (core func $i "run")))
  )
  (instance $d (instantiate $Driver
    (with "t" (type $t))
    (with "ctor" (func $ctor))
    (with "dtor1" (func $dtor1))
    (with "dtor2" (func $i1 "dtor"))
    (with "dtor3" (func $i2 "dtor"))
  ))
  (export "run" (func $d "run"))
)

(assert_return (invoke "run"))

;; A borrow can be passed back to the host multiple times before it's dropped.
(component definition $C
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1"
      (func (param "r" u32) (result (own $r))))
    (export "[method]resource1.simple"
      (func (param "self" (borrow $r)) (param "rep" u32)))
    (export "[static]resource1.assert"
      (func (param "r" (own $r)) (param "rep" u32)))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[method]resource1.simple" (func $simple))
  (alias export $host "[static]resource1.assert" (func $assert))

  ;; `f` passes its borrow to the host twice, then drops it.
  (component $B
    (import "r" (type $r (sub resource)))
    (import "simple"
      (func $simple (param "self" (borrow $r)) (param "rep" u32)))

    (core func $simple (canon lower (func $simple)))
    (core func $resource.drop (canon resource.drop $r))

    (core module $m
      (import "" "simple" (func $simple (param i32 i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (func (export "f") (param i32)
        (call $simple (local.get 0) (i32.const 100))
        (call $simple (local.get 0) (i32.const 100))
        (call $resource.drop (local.get 0)))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "simple" (func $simple))
        (export "resource.drop" (func $resource.drop))
      ))
    ))
    (func (export "f") (param "x" (borrow $r))
      (canon lift (core func $i "f")))
  )
  (instance $b (instantiate $B
    (with "r" (type $r))
    (with "simple" (func $simple))
  ))

  ;; `f` is given a borrow of a host resource and returns it as an `own`.
  (component $Ret
    (import "r" (type $r (sub resource)))
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "x" (borrow $r)) (result (own $r))
      (canon lift (core func $i "f")))
  )
  (instance $ret (instantiate $Ret (with "r" (type $r))))

  ;; `f` passes its borrow to a host function which takes an `own`.
  (component $Own
    (import "r" (type $r (sub resource)))
    (import "take-own" (func $take-own (param "r" (own $r)) (param "rep" u32)))
    (core func $take-own (canon lower (func $take-own)))
    (core module $m
      (import "" "take-own" (func $take-own (param i32 i32)))
      (func (export "f") (param i32)
        (call $take-own (local.get 0) (i32.const 100)))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "take-own" (func $take-own))
      ))
    ))
    (func (export "f") (param "x" (borrow $r))
      (canon lift (core func $i "f")))
  )
  (instance $own (instantiate $Own
    (with "r" (type $r))
    (with "take-own" (func $assert))
  ))

  (core func $ctor (canon lower (func $ctor)))
  (core func $b (canon lower (func $b "f")))
  (core func $ret (canon lower (func $ret "f")))
  (core func $own (canon lower (func $own "f")))
  (core func $resource.drop (canon resource.drop $r))

  (core module $m
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "b" (func $b (param i32)))
    (import "" "ret" (func $ret (param i32) (result i32)))
    (import "" "own" (func $own (param i32)))
    (import "" "resource.drop" (func $resource.drop (param i32)))
    (func (export "thread-through-borrow")
      (local $r i32)
      (local.set $r (call $ctor (i32.const 100)))
      (call $b (local.get $r))
      (call $resource.drop (local.get $r)))
    (func (export "return-borrow-as-own")
      (drop (call $ret (call $ctor (i32.const 100)))))
    (func (export "pass-borrow-as-own")
      (call $own (call $ctor (i32.const 100))))
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "ctor" (func $ctor))
      (export "b" (func $b))
      (export "ret" (func $ret))
      (export "own" (func $own))
      (export "resource.drop" (func $resource.drop))
    ))
  ))
  (func (export "thread-through-borrow")
    (canon lift (core func $i "thread-through-borrow")))
  (func (export "return-borrow-as-own")
    (canon lift (core func $i "return-borrow-as-own")))
  (func (export "pass-borrow-as-own")
    (canon lift (core func $i "pass-borrow-as-own")))
)

(component instance $C $C)
(assert_return (invoke "thread-through-borrow"))
;; The remaining borrow is reported, not the own/borrow mismatch.
(component instance $C $C)
(assert_trap (invoke "return-borrow-as-own")
  "borrow handles still remain at the end of the call")
(component instance $C $C)
(assert_trap (invoke "pass-borrow-as-own")
  "cannot lift own resource from a borrow")

;; An owned resource can't be dropped while it's lent to a pending async
;; host call.
(component
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1"
      (func (param "r" u32) (result (own $r))))
    (export "[method]resource1.never-return"
      (func async (param "self" (borrow $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[method]resource1.never-return" (func $never-return))

  (core func $ctor (canon lower (func $ctor)))
  (core func $never-return (canon lower (func $never-return) async))
  (core func $resource.drop (canon resource.drop $r))

  (core module $m
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "never-return" (func $never-return (param i32) (result i32)))
    (import "" "resource.drop" (func $resource.drop (param i32)))
    (func (export "f")
      (local $r i32)
      (local.set $r (call $ctor (i32.const 300)))
      ;; Expect the subtask to be STARTED.
      (if (i32.ne (i32.const 1)
            (i32.and (call $never-return (local.get $r)) (i32.const 0xf)))
        (then unreachable))
      (call $resource.drop (local.get $r))
      unreachable)
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "ctor" (func $ctor))
      (export "never-return" (func $never-return))
      (export "resource.drop" (func $resource.drop))
    ))
  ))
  (func (export "f") async (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "cannot remove owned resource while borrowed")

;; `resource.new` and `resource.rep` are lifted directly as exports, so the
;; host calls their trampolines. `rep` takes a raw handle index here, and 42
;; (the rep given to `new`) isn't one. A trap in `new` would poison the
;; instance, and the `assert_trap` would then see a different message.
(component
  (type $r' (resource (rep i32)))
  (export $r "r" (type $r'))

  (core func $resource.new (canon resource.new $r))
  (core func $resource.rep (canon resource.rep $r))

  (func (export "new") (param "x" u32) (result (own $r))
    (canon lift (core func $resource.new)))
  (func (export "rep") (param "x" u32) (result u32)
    (canon lift (core func $resource.rep)))
)

(invoke "new" (u32.const 42))
(assert_trap (invoke "rep" (u32.const 42)) "unknown handle index 42")

;; A task drops a borrow it was given while a subtask it started is blocked in
;; a sync-lowered async host call. The borrow is tracked by the dropping task,
;; not the blocked one.
(component
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1"
      (func (param "r" u32) (result (own $r))))
    (export "never-return" (func async))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "never-return" (func $never-return))

  ;; `call` blocks forever in a sync-lowered call to the host.
  (component $A
    (import "never-return" (func $never-return async))
    (core func $never-return (canon lower (func $never-return)))
    (core module $m
      (import "" "never-return" (func $never-return))
      (func (export "call") (result i32)
        (call $never-return)
        i32.const 0 (; EXIT ;))
      (func (export "callback") (param i32 i32 i32) (result i32)
        unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "never-return" (func $never-return))
      ))
    ))
    (func (export "call") async
      (canon lift (core func $i "call")
        async (callback (core func $i "callback"))))
  )
  (instance $a (instantiate $A (with "never-return" (func $never-return))))

  ;; `call` starts a subtask in `$A`, then drops its borrow and returns
  ;; without waiting for the subtask.
  (component $B
    (import "r" (type $r (sub resource)))
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core func $task.return (canon task.return))
    (core func $resource.drop (canon resource.drop $r))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "task.return" (func $task.return))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (func (export "call") (param i32) (result i32)
        ;; Expect the subtask to be STARTED.
        (if (i32.ne (i32.const 1) (i32.and (call $f) (i32.const 0xf)))
          (then unreachable))
        (call $resource.drop (local.get 0))
        (call $task.return)
        i32.const 0 (; EXIT ;))
      (func (export "callback") (param i32 i32 i32) (result i32)
        unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $f))
        (export "task.return" (func $task.return))
        (export "resource.drop" (func $resource.drop))
      ))
    ))
    (func (export "call") async (param "x" (borrow $r))
      (canon lift (core func $i "call")
        async (callback (core func $i "callback"))))
  )
  (instance $b (instantiate $B
    (with "r" (type $r))
    (with "f" (func $a "call"))
  ))

  (core func $ctor (canon lower (func $ctor)))
  (core func $call (canon lower (func $b "call")))
  (core func $resource.drop (canon resource.drop $r))
  (core module $m
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "call" (func $call (param i32)))
    (import "" "resource.drop" (func $resource.drop (param i32)))
    (func (export "run")
      (local $r i32)
      (local.set $r (call $ctor (i32.const 3)))
      (call $call (local.get $r))
      (call $resource.drop (local.get $r)))
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "ctor" (func $ctor))
      (export "call" (func $call))
      (export "resource.drop" (func $resource.drop))
    ))
  ))
  (func (export "run") async (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))

;; Drops a preload-owned handle whose destructor throws. The run must fail with
;; a trap about an exception leaving a component, not a propagated exception.
(component
  (import "dep" (instance $dep
    (export "res" (instance
      (export "thing" (type (sub resource)))
      (export "make" (func (result (own 0))))))))
  (alias export $dep "res" (instance $res))
  (alias export $res "thing" (type $t))

  (core func $make (canon lower (func $res "make")))
  (core func $drop (canon resource.drop $t))

  (core module $m
    (import "" "make" (func $make (result i32)))
    (import "" "drop" (func $drop (param i32)))
    (func (export "run") (result i32)
      (call $drop (call $make))
      (i32.const 0))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "make" (func $make))
    (export "drop" (func $drop))))))

  (func (export "run") (result u32) (canon lift (core func $i "run")))
)

;; A component whose `f` returns an owned resource, which WAVE can't render.
(component
  (type $r (resource (rep i32)))
  (export $r' "r" (type $r))
  (core func $new (canon resource.new $r))
  (core module $m
    (import "" "new" (func $new (param i32) (result i32)))
    (func (export "f") (result i32) (call $new (i32.const 1))))
  (core instance $i (instantiate $m (with "" (instance (export "new" (func $new))))))
  (func $f (result (own $r)) (canon lift (core func $i "f")))
  (export "f" (func $f) (func (result (own $r')))))

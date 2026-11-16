;;! flags = "-S max-component-handles=1000"

;; A configured handle limit above the one in `handle-table-limit.wast` admits
;; a spray that one rejects, and is itself enforced: it's below the default
;; limit, so a spray past it still traps.
(component definition $Tester
  (type $r (resource (rep i32)))
  (core func $resource.new (canon resource.new $r))

  (core module $m
    (import "" "resource.new" (func $resource.new (param i32) (result i32)))
    (func (export "spray") (param $n i32) (result i32)
      (local $i i32)
      (loop $l
        (if (i32.lt_u (local.get $i) (local.get $n))
          (then
            (drop (call $resource.new (local.get $i)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $l))))
      (local.get $i)))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "resource.new" (func $resource.new))
    ))
  ))

  (func (export "spray") (param "n" u32) (result u32)
    (canon lift (core func $i "spray"))))

(component instance $t $Tester)
(assert_return (invoke "spray" (u32.const 50)) (u32.const 50))

(component instance $t $Tester)
(assert_trap (invoke "spray" (u32.const 1001)) "store handle limit exceeded")

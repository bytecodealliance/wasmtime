;;! component_model_async = true

(component definition $A
  (core func $inc (canon backpressure.inc))
  (core func $dec (canon backpressure.dec))
  (core module $m
    (import "" "inc" (func $inc))
    (import "" "dec" (func $dec))
    (func (export "run") (param $inc i32) (param $dec i32)
      loop $l
        call $inc
        (local.tee $inc (i32.sub (local.get $inc) (i32.const 1)))
        if br $l end
      end

      loop $l
        call $dec
        (local.tee $dec (i32.sub (local.get $dec) (i32.const 1)))
        if br $l end
      end)
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "inc" (func $inc))
      (export "dec" (func $dec))
    ))
  ))
  (func (export "run") (param "incs" u32) (param "decs" u32)
    (canon lift (core func $i "run")))
)

(component instance $a1 $A)
(assert_trap (invoke "run" (u32.const 0) (u32.const 1)) "backpressure counter overflow")

(component instance $a2 $A)
(assert_trap (invoke "run" (u32.const 1) (u32.const 2)) "backpressure counter overflow")

(component instance $a3 $A)
(assert_trap (invoke "run" (u32.const 65536) (u32.const 0)) "backpressure counter overflow")

(component instance $a4 $A)
(assert_return (invoke "run" (u32.const 65535) (u32.const 65535)))
(assert_trap (invoke "run" (u32.const 0) (u32.const 1)) "backpressure counter overflow")

;;! component_model_async = true

;; NB: currently a copy of the test added in
;; https://github.com/WebAssembly/component-model/pull/666

(component definition $Tester
  (core module $Memory (memory (export "mem") 1))
  (core instance $memory (instantiate $Memory))
  (core module $M
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "stream.new" (func $stream.new (result i64)))

    (func $return-future-in-set (export "return-future-in-set") (result i32)
      (local $ret64 i64) (local $rx i32) (local $ws i32)
      (local.set $ret64 (call $future.new))
      (local.set $rx (i32.wrap_i64 (local.get $ret64)))
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get $rx) (local.get $ws))
      (local.get $rx)
    )
    (func $return-stream-in-set (export "return-stream-in-set") (result i32)
      (local $ret64 i64) (local $rx i32) (local $ws i32)
      (local.set $ret64 (call $stream.new))
      (local.set $rx (i32.wrap_i64 (local.get $ret64)))
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get $rx) (local.get $ws))
      (local.get $rx)
    )
  )
  (type $FT (future u8))
  (type $ST (stream u8))
  (canon waitable.join (core func $waitable.join))
  (canon waitable-set.new (core func $waitable-set.new))
  (canon future.new $FT (core func $future.new))
  (canon stream.new $ST (core func $stream.new))
  (core instance $m (instantiate $M (with "" (instance
    (export "waitable.join" (func $waitable.join))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "future.new" (func $future.new))
    (export "stream.new" (func $stream.new))
  ))))
  (func (export "return-future-in-set") async (result $FT) (canon lift (core func $m "return-future-in-set")))
  (func (export "return-stream-in-set") async (result $ST) (canon lift (core func $m "return-stream-in-set")))
)

(component instance $i1 $Tester)
(assert_trap (invoke "return-future-in-set") "cannot lift future while it's in a waitable set")
(component instance $i2 $Tester)
(assert_trap (invoke "return-stream-in-set") "cannot lift stream while it's in a waitable set")

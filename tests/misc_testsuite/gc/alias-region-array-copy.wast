;;! gc = true

;; Regression test for an alias-region miscompile in the inline `array.copy`
;; fast path (`emit_inline_memcpy`). A small constant-length copy of an `i32`
;; array must tag its inline loads/stores with the GC-heap alias region;
;; otherwise the region-less load can be store-to-load forwarded a stale value
;; across the intervening region-tagged `array.set`.
;;
;; The array is passed as a function parameter so that the two `array.copy`
;; element addresses share a single SSA value (a fresh `array.new` local or a
;; `global.get` reloads the reference and hides the forwarding).
(module
  (type $a (array (mut i32)))
  (func $op (param $arr (ref $a)) (result i32)
    (array.set $a (local.get $arr) (i32.const 4) (i32.const 0x1111))
    (array.copy $a $a (local.get $arr) (i32.const 0) (local.get $arr) (i32.const 4) (i32.const 1))
    (array.set $a (local.get $arr) (i32.const 0) (i32.const 0x2222))
    (array.copy $a $a (local.get $arr) (i32.const 2) (local.get $arr) (i32.const 0) (i32.const 1))
    (array.get $a (local.get $arr) (i32.const 2)) ;; must be 0x2222
  )
  (func (export "test") (result i32)
    (call $op (array.new $a (i32.const 0) (i32.const 8)))
  )
)
(assert_return (invoke "test") (i32.const 0x2222))

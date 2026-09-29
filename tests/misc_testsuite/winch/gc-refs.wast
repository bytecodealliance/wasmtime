;;! reference_types = true
;;! gc_types = true

;; Winch reference-value support: externref flows through params, results,
;; locals, and globals, and survives calls (stack maps). Global reads and
;; writes use barriers when running under the DRC collector.

(module
  (global $g (mut externref) (ref.null extern))

  (func $nop)

  (func (export "roundtrip") (param externref) (result externref)
    (local $l externref)
    (local.set $l (local.get 0))
    (call $nop)
    (local.get $l))

  (func (export "via_global") (param externref) (result externref)
    (global.set $g (local.get 0))
    (global.get $g))

  (func (export "null_is_null") (result i32)
    (ref.is_null (global.get $g))))

(assert_return (invoke "null_is_null") (i32.const 1))
(assert_return (invoke "roundtrip" (ref.null extern)) (ref.null extern))
(assert_return (invoke "via_global" (ref.null extern)) (ref.null extern))

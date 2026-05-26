;;! gc = true

;; Regression test for an `array.copy` whose destination slot holds an
;; `i31ref` and whose source is a non-`i31ref` GC object. The write
;; barrier filter (`GcStore::needs_write_barrier`) fires because the
;; source is a real GC object, so the call reaches the heap's
;; `write_gc_ref` with `dest = Some(i31ref)`. The DRC collector
;; previously asserted `!gc_ref.is_i31()` inside
;; `dec_ref_and_maybe_dealloc`, panicking instead of treating the
;; `i31ref` as a no-op.

(module
  (type $box (struct (field i32)))
  (type $arr (array (mut anyref)))

  (func (export "test")
    (local $src (ref $arr))
    (local $dst (ref $arr))
    ;; Source: real GC objects (`structref`s).
    (local.set $src
      (array.new_fixed $arr 3
        (struct.new $box (i32.const 1))
        (struct.new $box (i32.const 2))
        (struct.new $box (i32.const 3))))
    ;; Destination: `i31ref`s. Each `array.copy` element write
    ;; overwrites an `i31ref` slot with a `structref`, exercising the
    ;; `dec_ref_and_maybe_dealloc(i31ref)` path.
    (local.set $dst
      (array.new $arr (ref.i31 (i32.const 42)) (i32.const 3)))
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (local.get $src) (i32.const 0)
      (i32.const 3))
  )
)

(assert_return (invoke "test"))

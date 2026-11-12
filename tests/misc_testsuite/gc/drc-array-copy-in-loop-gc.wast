;;! gc = true
;;! flags = """
;;!   -C collector=drc
;;!   -O gc-heap-reservation=131072
;;!   -O gc-heap-reservation-for-growth=0
;;! """

;; Regression test for a stack-map bug in `translate_per_element_copy`, which
;; implements `array.copy` on arrays of GC references.
;;
;; The per-element loop computed raw element addresses once and used only
;; those inside the loop, so the source and destination arrays were dead in
;; CLIF there and missing from stack maps at its safe points. Once the DRC
;; collector's read barrier started calling `force_gc` (when the
;; over-approximated-stack-roots list grows past 1024 entries), a GC could run
;; mid-copy with neither array rooted, free them, and cascade dec-refs through
;; their elements, after which the copy read freed memory.
;;
;; The small GC heap makes freed slots get reused quickly, so a stale element
;; is observable.
(module
  (import "wasmtime" "gc" (func $gc))
  (type $box (struct (field i32)))
  (type $arr (array (mut (ref null $box))))

  ;; The `array.copy` is in a helper so the source and destination arrays are
  ;; parameters used only to compute the initial addresses, and so are dead
  ;; inside the per-element loop.
  (func $do_copy
    (param $src (ref null $arr))
    (param $dst (ref null $arr))
    (param $len i32)
    (array.copy $arr $arr
      (local.get $dst) (i32.const 0)
      (local.get $src) (i32.const 0)
      (local.get $len)))

  (func (export "test") (result i32)
    (local $src (ref null $arr))
    (local $dst (ref null $arr))
    (local $i i32)
    (local $sum i32)

    ;; 2048 > 1024, so the per-element read barrier fires `force_gc` during
    ;; the copy.
    (local.set $src
      (array.new $arr (ref.null $box) (i32.const 2048)))
    (local.set $i (i32.const 0))
    (loop $fill
      (array.set $arr (local.get $src) (local.get $i)
        (struct.new $box (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $fill (i32.lt_s (local.get $i) (i32.const 2048))))

    (local.set $dst
      (array.new $arr (ref.null $box) (i32.const 2048)))

    (call $gc)

    (call $do_copy (local.get $src) (local.get $dst) (i32.const 2048))

    ;; Drop the source and collect, so elements wrongly freed while still
    ;; reachable from $dst show up as a trap or wrong contents below.
    (local.set $src (ref.null $arr))
    (call $gc)

    ;; Allocate heavily to reuse freed slots. A stale element of $dst would
    ;; then read back as -1 rather than its index.
    (local.set $i (i32.const 0))
    (loop $churn
      (drop (struct.new $box (i32.const -1)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $churn (i32.lt_s (local.get $i) (i32.const 4096))))

    (call $gc)

    (local.set $i (i32.const 0))
    (local.set $sum (i32.const 0))
    (loop $verify
      (local.set $sum
        (i32.add (local.get $sum)
          (struct.get $box 0
            (array.get $arr (local.get $dst) (local.get $i)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $verify (i32.lt_s (local.get $i) (i32.const 2048))))

    (local.get $sum)))

;; The sum of 0..2048 is 2048 * 2047 / 2.
(assert_return (invoke "test") (i32.const 2096128))

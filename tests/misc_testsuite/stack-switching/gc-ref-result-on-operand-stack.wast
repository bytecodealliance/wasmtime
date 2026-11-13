;;! stack_switching = true
;;! gc = true
;;! bulk_memory = true
;;! flags = "-C collector=copying -O gc-zeal-alloc-counter=1"

;; https://github.com/bytecodealliance/wasmtime/issues/13750
;;
;; A GC reference returned through a continuation's result must keep its
;; stack-map entry while it is live on the operand stack across later GC
;; safepoints.
(module
  (type $S (struct (field $x i32)))
  (type $A (array i64))
  (type $ft (func (result (ref $S))))
  (type $ct (cont $ft))

  (func $target (result (ref $S))
    (struct.new $S (i32.const 0x12345678)))
  (elem declare func $target)

  (func (export "run") (result i32)
    (local $k (ref null $ct))
    (local $i i32)
    (local.set $k (cont.new $ct (ref.func $target)))

    ;; Keep the continuation's GC-reference result on the operand stack,
    ;; rather than storing it in a GC-typed local.
    (resume $ct (local.get $k))

    ;; Force collections while that operand-stack value is the only live
    ;; reference to the struct.
    (local.set $i (i32.const 0))
    (loop $again
      (drop (array.new_default $A (i32.const 256)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $again
        (i32.lt_u (local.get $i) (i32.const 2000))))

    (struct.get $S $x)))

(assert_return (invoke "run") (i32.const 0x12345678))

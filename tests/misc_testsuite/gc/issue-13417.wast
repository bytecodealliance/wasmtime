;;! gc = true

(module)

(thread $old
  (module
    ;; Register stale trace metadata for a large struct whose final field is a
    ;; GC reference. The field offset is valid for this type, but not for the
    ;; smaller type instantiated after this thread's Store is dropped.
    (type $old (struct
      (field i64) (field i64) (field i64) (field i64)
      (field i64) (field i64) (field i64) (field i64)
      (field i64) (field i64) (field i64) (field i64)
      (field i64) (field i64) (field i64) (field i64)
      (field anyref)))
    (global (ref null $old)
      (struct.new $old
        (i64.const 0) (i64.const 0) (i64.const 0) (i64.const 0)
        (i64.const 0) (i64.const 0) (i64.const 0) (i64.const 0)
        (i64.const 0) (i64.const 0) (i64.const 0) (i64.const 0)
        (i64.const 0) (i64.const 0) (i64.const 0) (i64.const 0)
        (ref.null any)))))
(wait $old)

(module
  (type $new (struct (field (mut i32))))
  (global $g (mut (ref null $new))
    (struct.new $new (i32.const 1)))
  (func (export "trigger")
    ;; Overwriting the global makes DRC decrement and deallocate the old value,
    ;; consuming the stale trace metadata without forcing an explicit GC.
    (global.set $g (ref.null $new))))

(invoke "trigger")

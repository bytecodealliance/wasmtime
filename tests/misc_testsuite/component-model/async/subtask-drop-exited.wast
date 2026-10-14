;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true

(component
  (component $C
    (canon task.return (core func $task.return))

    (core module $M
      (import "" "task.return" (func $task.return))
      (global $yield-count (mut i32) (i32.const 0))
      (func (export "f") (result i32)
        (i32.const 1 (; YIELD ;)))
      (func (export "f-cb") (param i32 i32 i32) (result i32)
        (if (i32.eq (global.get $yield-count) (i32.const 0))
            (then (call $task.return)))
        (global.set $yield-count (i32.add (global.get $yield-count) (i32.const 1)))
        (if (result i32) (i32.ge_u (global.get $yield-count) (i32.const 5))
            (then (i32.const 0 (; EXIT ;)))
            (else (i32.const 1 (; YIELD ;)))))
    )

    (core instance $m (instantiate $M (with "" (instance
      (export "task.return" (func $task.return))))))
    (func $f (export "f") async (canon lift (core func $m "f") async (callback (core func $m "f-cb"))))
  )

  (component $D
    (import "f" (func $f async))
    (canon lower (func $f) async (core func $f))
    (canon task.return (core func $task.return))
    (canon subtask.drop (core func $subtask.drop))
    (canon subtask.cancel (core func $subtask.cancel))

    (core module $M
      (import "" "task.return" (func $task.return))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "f" (func $f (result i32)))
      (global $subtask (mut i32) (i32.const 0))
      (global $yield-count (mut i32) (i32.const 0))
      (func (export "f") (result i32)
        (local $result i32)
        (local.set $result (call $f))
        (if (i32.ne (i32.and (local.get $result) (i32.const 0xf)) (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (global.set $subtask (i32.shr_u (local.get $result) (i32.const 4)))
        (i32.const 1 (; YIELD ;)))
      (func (export "f-cb") (param i32 i32 i32) (result i32)
        (if (i32.eq (global.get $yield-count) (i32.const 0))
          (then
            (if (i32.ne (call $subtask.cancel (global.get $subtask)) (i32.const 2 (; RETURNED ;)))
              (then unreachable))))
        (global.set $yield-count (i32.add (global.get $yield-count) (i32.const 1)))
        (if (result i32) (i32.ge_u (global.get $yield-count) (i32.const 10))
            (then
              (call $subtask.drop (global.get $subtask))
              (call $task.return)
              (i32.const 0 (; EXIT ;)))
            (else (i32.const 1 (; YIELD ;)))))
    )

    (core instance $m (instantiate $M (with "" (instance
      (export "f" (func $f))
      (export "task.return" (func $task.return))
      (export "subtask.drop" (func $subtask.drop))
      (export "subtask.cancel" (func $subtask.cancel))))))
    (func (export "f") async (canon lift (core func $m "f") async (callback (core func $m "f-cb"))))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D
    (with "f" (func $c "f"))
  ))
  (func (export "run") (alias export $d "f"))
)

(assert_return (invoke "run"))

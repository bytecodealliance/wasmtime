;;! component_model_async = true
;;! component_model_threading = true

(component
  (import "host" (instance $host
    (export "never-return" (func async))
  ))

  (core module $m
    (import "" "never-return" (func $never-return (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "drop-subtask" (func $drop-subtask (param i32)))
    (import "" "thread.new-indirect" (func $thread-new (param i32 i32) (result i32)))
    (import "" "thread.index" (func $thread-index (result i32)))
    (import "" "thread.suspend-then-resume" (func $thread-suspend-then-resume (param i32) (result i32)))
    (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
    (import "" "table" (table $table 1 funcref))

    (global $main-thread (mut i32) (i32.const 0))
    (global $subtask (mut i32) (i32.const 0))
    (global $done (mut i32) (i32.const 0))

    (elem (i32.const 0) $creator)

    (func (export "run")
      (global.set $main-thread (call $thread-index))

      ;; Spawn a thread to start a task.
      (drop (call $thread-suspend-then-resume
        (call $thread-new (i32.const 0) (i32.const 0))))
      (if (i32.ne (global.get $done) (i32.const 1)) (then unreachable))

      ;; Should be able to cancel the subtask on this thread.
      (i32.ne
        (call $cancel (global.get $subtask))
        (i32.const 4)) ;; RETURN_CANCELLED
      if unreachable end
      (call $drop-subtask (global.get $subtask))
    )

    ;; Start an import call then exit this thread.
    (func $creator (param i32)
      (local $ret i32)
      (local.set $ret (call $never-return))

      (i32.ne
        (i32.and (local.get $ret) (i32.const 0xf))
        (i32.const 1)) ;; STARTED
      if unreachable end

      (global.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
      (global.set $done (i32.const 1))
      (call $thread-resume-later (global.get $main-thread))
    )
  )

  (core module $libc (table (export "table") 1 funcref))
  (core instance $libc (instantiate $libc))
  (alias core export $libc "table" (core table $table))
  (core type $start-func-ty (func (param i32)))

  (core func $never-return (canon lower (func $host "never-return") async))
  (core func $cancel (canon subtask.cancel))
  (core func $drop-subtask (canon subtask.drop))
  (core func $thread.new-indirect (canon thread.new-indirect $start-func-ty (core table $table)))
  (core func $thread.index (canon thread.index))
  (core func $thread.suspend-then-resume (canon thread.suspend-then-resume))
  (core func $thread.resume-later (canon thread.resume-later))
  (core instance $i (instantiate $m
      (with "" (instance
          (export "never-return" (func $never-return))
          (export "cancel" (func $cancel))
          (export "drop-subtask" (func $drop-subtask))
          (export "thread.new-indirect" (func $thread.new-indirect))
          (export "thread.index" (func $thread.index))
          (export "thread.suspend-then-resume" (func $thread.suspend-then-resume))
          (export "thread.resume-later" (func $thread.resume-later))
          (export "table" (table $table))
      ))
  ))

  (func (export "f") async
      (canon lift (core func $i "run")))
)

(assert_return (invoke "f"))

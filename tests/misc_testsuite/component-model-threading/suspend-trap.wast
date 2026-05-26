;;! component_model_async = true
;;! component_model_threading = true

;; Test that a thread which is not suspended cannot be resumed
(component
    (core module $libc
        (memory (export "mem") 1)
        (table (export "table") 2 funcref)
    )
    (core module $CM
        (import "" "thread.new-indirect" (func $thread.new-indirect (param i32 i32) (result i32)))
        (import "" "thread.yield-to-suspended" (func $thread.yield-to-suspended (param i32) (result i32)))
        (import "" "thread.yield" (func $thread.yield (result i32)))
        (import "libc" "mem" (memory 1))
        (import "libc" "table" (table $table 2 funcref))

        (func (export "run")
          (local $id i32)
          ;; start `$id` suspended
          (local.set $id (call $thread.new-indirect (i32.const 0) (i32.const 0)))

          ;; Resume it, which will come back here due to `$thread.yield`
          (drop (call $thread.yield-to-suspended (local.get $id)))

          ;; try to resume it again and this should trap.
          (drop (call $thread.yield-to-suspended (local.get $id)))
        )

        (func $child (param i32)
          (drop (call $thread.yield))
        )
        (elem (table $table) (i32.const 0) func $child)
    )

    (core instance $libc (instantiate $libc))
    (core type $start-func-ty (func (param i32)))

    (core func $task-cancel (canon task.cancel))
    (core func $thread-new-indirect
        (canon thread.new-indirect $start-func-ty (table $libc "table")))
    (core func $thread-yield (canon thread.yield))
    (core func $thread-yield-to-suspended (canon thread.yield-to-suspended))

    (core instance $cm (instantiate $CM
        (with "" (instance
            (export "thread.new-indirect" (func $thread-new-indirect))
            (export "thread.yield-to-suspended" (func $thread-yield-to-suspended))
            (export "thread.yield" (func $thread-yield))
        ))
        (with "libc" (instance $libc))
    ))

    (func (export "run") async (canon lift (core func $cm "run")))
)

(assert_trap (invoke "run") "cannot resume thread which is not suspended")

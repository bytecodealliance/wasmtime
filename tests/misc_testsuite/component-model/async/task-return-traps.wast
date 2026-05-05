;;! component_model_async = true
;;! component_model_async_stackful = true
;;! component_model_threading = true

(component
    (core module $m
        (import "" "task.return" (func $task-return))
        (func (export "foo") (result i32)
            i32.const 0
        )
        (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
    )
    (core func $task-return (canon task.return))
    (core instance $i (instantiate $m
        (with "" (instance (export "task.return" (func $task-return))))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async (callback (func $i "callback"))))
)
(assert_trap (invoke "foo") "async-lifted export failed to produce a result")

(component
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref))
    (core module $m
        (import "" "task.return" (func $task-return))
        (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.unsuspend" (func $thread-unsuspend (param i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))
        (func $thread-start (param i32) (; empty ;))
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)
        (func (export "foo") (result i32)
            (call $thread-unsuspend
                (call $thread-new-indirect (i32.const 0) (i32.const 0)))
            i32.const 0
        )
        (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $libc (instantiate $libc))
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))
    (core func $thread-new-indirect
        (canon thread.new-indirect $start-func-ty (table $indirect-function-table)))
    (core func $thread-unsuspend (canon thread.unsuspend))
    (core func $task-return (canon task.return))
    (core instance $i (instantiate $m
        (with "" (instance
            (export "thread.new-indirect" (func $thread-new-indirect))
            (export "thread.unsuspend" (func $thread-unsuspend))
            (export "task.return" (func $task-return))
        ))
        (with "libc" (instance $libc))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async (callback (func $i "callback"))))
)

(assert_trap (invoke "foo") "async-lifted export failed to produce a result")

(component
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref))
    (core module $m
        (import "" "task.return" (func $task-return))
        (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.unsuspend" (func $thread-unsuspend (param i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))
        (func $thread-start (param i32) (; empty ;))
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)
        (func (export "foo")
            (call $thread-unsuspend
                (call $thread-new-indirect (i32.const 0) (i32.const 0)))
        )
    )
    (core instance $libc (instantiate $libc))
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))
    (core func $thread-new-indirect
        (canon thread.new-indirect $start-func-ty (table $indirect-function-table)))
    (core func $thread-unsuspend (canon thread.unsuspend))
    (core func $task-return (canon task.return))
    (core instance $i (instantiate $m
        (with "" (instance
            (export "thread.new-indirect" (func $thread-new-indirect))
            (export "thread.unsuspend" (func $thread-unsuspend))
            (export "task.return" (func $task-return))
        ))
        (with "libc" (instance $libc))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async))
)

(assert_trap (invoke "foo") "async-lifted export failed to produce a result")

(component
    (core module $m
        (import "" "task.return" (func $task-return))
        (func (export "foo"))
    )
    (core func $task-return (canon task.return))
    (core instance $i (instantiate $m
        (with "" (instance (export "task.return" (func $task-return))))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async))
)
(assert_trap (invoke "foo") "async-lifted export failed to produce a result")

(component
    (core module $m
        (import "" "task.return" (func $task-return (param i32)))
        (func (export "foo") (call $task-return (i32.const 42)))
    )
    (core func $task-return (canon task.return (result u32)))
    (core instance $i (instantiate $m
        (with "" (instance (export "task.return" (func $task-return))))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async))
)

(assert_trap (invoke "foo")
  "invalid `task.return` signature and/or options for current task")

(component
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core module $m
        (import "" "task.return" (func $task-return))
        (func (export "foo") (call $task-return))
    )
    (core func $task-return (canon task.return (memory $libc "memory")))
    (core instance $i (instantiate $m
        (with "" (instance (export "task.return" (func $task-return))))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async))
)

(assert_trap (invoke "foo")
  "invalid `task.return` signature and/or options for current task")

(component
    (core module $m
        (import "" "task.return" (func $task-return))
        (func (export "foo") (call $task-return))
    )
    (core func $task-return (canon task.return string-encoding=utf16))
    (core instance $i (instantiate $m
        (with "" (instance (export "task.return" (func $task-return))))
    ))
    (func (export "foo") (canon lift (core func $i "foo") async))
)

(assert_trap (invoke "foo")
  "invalid `task.return` signature and/or options for current task")

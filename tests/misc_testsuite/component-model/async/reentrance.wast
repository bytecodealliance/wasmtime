;;! component_model_async = true
;;! reference_types = true

(component
    (core module $shim
        (import "" "task.return" (func $task-return (param i32)))
        (table (export "funcs") 1 1 funcref)
        (func (export "export") (param i32) (result i32)
            (call_indirect (i32.const 0) (local.get 0))
        )
        (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
    )
    (core func $task-return (canon task.return (result u32)))
    (core instance $shim (instantiate $shim
        (with "" (instance (export "task.return" (func $task-return))))
    ))
    (func $shim-export (param "p1" u32) (result u32)
        (canon lift (core func $shim "export") async (callback (func $shim "callback")))
    )

    (component $inner
        (import "import" (func $import (param "p1" u32) (result u32)))
        (core module $libc (memory (export "memory") 1))
        (core instance $libc (instantiate $libc))
        (core func $import (canon lower (func $import) async (memory $libc "memory")))

        (core module $m
            (import "libc" "memory" (memory 1))
            (import "" "import" (func $import (param i32 i32) (result i32)))
            (import "" "task.return" (func $task-return (param i32)))
            (func (export "export") (param i32) (result i32)
                (i32.store offset=0 (i32.const 1200) (local.get 0))
                (call $import (i32.const 1200) (i32.const 1204))
                drop
                (call $task-return (i32.load offset=0 (i32.const 1204)))
                i32.const 0
            )
            (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
        )
        (core type $task-return-type (func (param i32)))
        (core func $task-return (canon task.return (result u32)))
        (core instance $i (instantiate $m
            (with "" (instance
                (export "task.return" (func $task-return))
                (export "import" (func $import))
            ))
            (with "libc" (instance $libc))
        ))
        (func (export "export") (param "p1" u32) (result u32)
            (canon lift (core func $i "export") async (callback (func $i "callback")))
        )
    )
    (instance $inner (instantiate $inner (with "import" (func $shim-export))))

    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $inner-export (canon lower (func $inner "export") async (memory $libc "memory")))

    (core module $donut
        (import "" "funcs" (table 1 1 funcref))
        (import "libc" "memory" (memory 1))
        (import "" "import" (func $import (param i32 i32) (result i32)))
        (import "" "task.return" (func $task-return (param i32)))
        (func $host-export (export "export") (param i32) (result i32)
            (i32.store offset=0 (i32.const 1200) (local.get 0))
            (call $import (i32.const 1200) (i32.const 1204))
            drop
            (call $task-return (i32.load offset=0 (i32.const 1204)))
            i32.const 0
        )
        (func $guest-export (export "guest-export") (param i32) (result i32) unreachable)
        (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
        (func $start
            (table.set (i32.const 0) (ref.func $guest-export))
        )
        (start $start)
    )

    (core instance $donut (instantiate $donut
        (with "" (instance
            (export "task.return" (func $task-return))
            (export "import" (func $inner-export))
            (export "funcs" (table $shim "funcs"))
        ))
        (with "libc" (instance $libc))
    ))
    (func (export "export") (param "p1" u32) (result u32)
        (canon lift (core func $donut "export") async (callback (func $donut "callback")))
    )
)

(assert_trap (invoke "export" (u32.const 42)) "cannot enter component instance")

;;! component_model_async = true
;;! component_model_async_stackful = true
;;! component_model_async_builtins = true
;;! component_model_threading = true
;;! reference_types = true

(component
    (component $C
        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        ;; Defines the table for the thread start functions, of which there are two
        (core module $libc
            (table (export "__indirect_function_table") 3 funcref))
        (core module $CM
            (import "" "mem" (memory 1))
            (import "" "task.return" (func $task-return (param i32)))
            (import "" "task.cancel" (func $task-cancel))
            (import "" "thread.new_indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
            (import "" "thread.suspend" (func $thread-suspend (result i32)))
            (import "" "thread.suspend-cancellable" (func $thread-suspend-cancellable (result i32)))
            (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
            (import "" "thread.yield-to-cancellable" (func $thread-yield-to-cancellable (param i32) (result i32)))
            (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
            (import "" "thread.switch-to-cancellable" (func $thread-switch-to-cancellable (param i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))
            (import "" "thread.yield-cancellable" (func $thread-yield-cancellable (result i32)))
            (import "" "thread.index" (func $thread-index (result i32)))
            (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "libc" "__indirect_function_table" (table $indirect-function-table 3 funcref))

            ;; Indices into the function table for the thread start functions
            (global $call-return-ftbl-idx i32 (i32.const 0))
            (global $suspend-ftbl-idx i32 (i32.const 1))
            (global $yield-loop-ftbl-idx i32 (i32.const 2))

            (func $call-return (param i32)
                (call $task-return (local.get 0)))

            (func $suspend (param i32)
                (drop (call $thread-suspend)))

            (func $yield-loop (param i32)
                (loop $top 
                    (drop (call $thread-yield)) 
                    (br $top)))

            (func (export "explicit-thread-calls-return-stackful")
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $call-return-ftbl-idx))))

            (func (export "explicit-thread-calls-return-stackless") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $call-return-ftbl-idx)))
                (i32.const 0 (; EXIT ;)))

            (func (export "cb") (param i32 i32 i32) (result i32)
                (unreachable))
            
            (func (export "explicit-thread-suspends-sync") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $suspend-ftbl-idx)))
                (i32.const 42))

            (func (export "explicit-thread-suspends-stackful")
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $suspend-ftbl-idx)))
                (call $task-return (i32.const 42)))

            (func (export "explicit-thread-suspends-stackless") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $suspend-ftbl-idx)))
                (call $task-return (i32.const 42))
                (i32.const 0))

            (func (export "explicit-thread-yield-loops-sync") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $yield-loop-ftbl-idx)))
                (i32.const 42))

            (func (export "explicit-thread-yield-loops-stackful")
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $yield-loop-ftbl-idx)))
                (call $task-return (i32.const 42)))

            (func (export "explicit-thread-yield-loops-stackless") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (i32.const 0) (global.get $suspend-ftbl-idx)))
                (call $task-return (i32.const 42))
                (i32.const 0 (; EXIT ;)))
            

            ;; Initialize the function table that will be used by thread.new_indirect
            (elem (table $indirect-function-table) (i32.const 0 (; call-return-ftbl-idx ;)) func $call-return)
            (elem (table $indirect-function-table) (i32.const 1 (; suspend-ftbl-idx ;)) func $suspend)
            (elem (table $indirect-function-table) (i32.const 2 (; yield-loop-ftbl-idx ;)) func $yield-loop)
        ) 

        ;; Instantiate the libc module to get the table
        (core instance $libc (instantiate $libc))
        ;; Get access to `thread.new_indirect` that uses the table from libc
        (core type $start-func-ty (func (param i32)))
        (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

        (core func $task-return (canon task.return (result u32)))
        (core func $task-cancel (canon task.cancel))
        (core func $thread-new-indirect 
            (canon thread.new_indirect $start-func-ty (table $indirect-function-table)))
        (core func $thread-yield (canon thread.yield))
        (core func $thread-yield-cancellable (canon thread.yield cancellable))
        (core func $thread-index (canon thread.index))
        (core func $thread-yield-to (canon thread.yield-to))
        (core func $thread-yield-to-cancellable (canon thread.yield-to cancellable))
        (core func $thread-resume-later (canon thread.resume-later))
        (core func $thread-switch-to (canon thread.switch-to))
        (core func $thread-switch-to-cancellable (canon thread.switch-to cancellable))
        (core func $thread-suspend (canon thread.suspend))
        (core func $thread-suspend-cancellable (canon thread.suspend cancellable))
        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable.join (canon waitable.join))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))

        ;; Instantiate the main module
        (core instance $cm (
            instantiate $CM
                (with "" (instance
                    (export "mem" (memory $memory "mem"))
                    (export "task.return" (func $task-return))
                    (export "task.cancel" (func $task-cancel))
                    (export "thread.new_indirect" (func $thread-new-indirect))
                    (export "thread.index" (func $thread-index))
                    (export "thread.yield-to" (func $thread-yield-to))
                    (export "thread.yield-to-cancellable" (func $thread-yield-to-cancellable))
                    (export "thread.yield" (func $thread-yield))
                    (export "thread.yield-cancellable" (func $thread-yield-cancellable))
                    (export "thread.switch-to" (func $thread-switch-to))
                    (export "thread.switch-to-cancellable" (func $thread-switch-to-cancellable))
                    (export "thread.suspend" (func $thread-suspend))
                    (export "thread.suspend-cancellable" (func $thread-suspend-cancellable))
                    (export "thread.resume-later" (func $thread-resume-later))
                    (export "waitable.join" (func $waitable.join))
                    (export "waitable-set.wait" (func $waitable-set.wait))
                    (export "waitable-set.new" (func $waitable-set.new))))
                (with "libc" (instance $libc))))

        (func (export "explicit-thread-calls-return-stackful") (result u32) 
            (canon lift (core func $cm "explicit-thread-calls-return-stackful") async))
        (func (export "explicit-thread-calls-return-stackless") (result u32) 
            (canon lift (core func $cm "explicit-thread-calls-return-stackless") async (callback (func $cm "cb"))))
        (func (export "explicit-thread-suspends-sync") (result u32) 
            (canon lift (core func $cm "explicit-thread-suspends-sync")))
        (func (export "explicit-thread-suspends-stackful") (result u32) 
            (canon lift (core func $cm "explicit-thread-suspends-stackful") async))
        (func (export "explicit-thread-suspends-stackless") (result u32) 
            (canon lift (core func $cm "explicit-thread-suspends-stackless") async (callback (func $cm "cb"))))
        (func (export "explicit-thread-yield-loops-sync") (result u32) 
            (canon lift (core func $cm "explicit-thread-yield-loops-sync")))
        (func (export "explicit-thread-yield-loops-stackful") (result u32) 
            (canon lift (core func $cm "explicit-thread-yield-loops-stackful") async))
        (func (export "explicit-thread-yield-loops-stackless") (result u32) 
            (canon lift (core func $cm "explicit-thread-yield-loops-stackless") async (callback (func $cm "cb"))))
    )

    (component $D 
        (import "explicit-thread-calls-return-stackful" (func $explicit-thread-calls-return-stackful (result u32)))
        (import "explicit-thread-calls-return-stackless" (func $explicit-thread-calls-return-stackless (result u32)))
        (import "explicit-thread-suspends-sync" (func $explicit-thread-suspends-sync (result u32)))
        (import "explicit-thread-suspends-stackful" (func $explicit-thread-suspends-stackful (result u32)))
        (import "explicit-thread-suspends-stackless" (func $explicit-thread-suspends-stackless (result u32)))
        (import "explicit-thread-yield-loops-sync" (func $explicit-thread-yield-loops-sync (result u32)))
        (import "explicit-thread-yield-loops-stackful" (func $explicit-thread-yield-loops-stackful (result u32)))
        (import "explicit-thread-yield-loops-stackless" (func $explicit-thread-yield-loops-stackless (result u32)))

        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        (core module $DM
            (import "" "mem" (memory 1))
            (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
            ;; sync lowered
            (import "" "explicit-thread-calls-return-stackful" (func $explicit-thread-calls-return-stackful (result i32)))
            (import "" "explicit-thread-calls-return-stackless" (func $explicit-thread-calls-return-stackless (result i32)))
            (import "" "explicit-thread-suspends-sync" (func $explicit-thread-suspends-sync (result i32)))
            (import "" "explicit-thread-suspends-stackful" (func $explicit-thread-suspends-stackful (result i32)))
            (import "" "explicit-thread-suspends-stackless" (func $explicit-thread-suspends-stackless (result i32)))
            (import "" "explicit-thread-yield-loops-sync" (func $explicit-thread-yield-loops-sync (result i32)))
            (import "" "explicit-thread-yield-loops-stackful" (func $explicit-thread-yield-loops-stackful (result i32)))
            (import "" "explicit-thread-yield-loops-stackless" (func $explicit-thread-yield-loops-stackless (result i32)))
            ;; async lowered
            (import "" "explicit-thread-calls-return-stackful-async" (func $explicit-thread-calls-return-stackful-async (param i32) (result i32)))
            (import "" "explicit-thread-calls-return-stackless-async" (func $explicit-thread-calls-return-stackless-async (param i32) (result i32)))
            (import "" "explicit-thread-suspends-sync-async" (func $explicit-thread-suspends-sync-async (param i32) (result i32)))
            (import "" "explicit-thread-suspends-stackful-async" (func $explicit-thread-suspends-stackful-async (param i32) (result i32)))
            (import "" "explicit-thread-suspends-stackless-async" (func $explicit-thread-suspends-stackless-async (param i32) (result i32)))
            (import "" "explicit-thread-yield-loops-sync-async" (func $explicit-thread-yield-loops-sync-async (param i32) (result i32)))
            (import "" "explicit-thread-yield-loops-stackful-async" (func $explicit-thread-yield-loops-stackful-async (param i32) (result i32)))
            (import "" "explicit-thread-yield-loops-stackless-async" (func $explicit-thread-yield-loops-stackless-async (param i32) (result i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))

            (func $check (param i32)
                (if (i32.ne (local.get 0) (i32.const 42))
                    (then unreachable))
            )

            (func $check-async (param i32)
                (local $retp i32) (local $ws i32) (local $ws-retp i32)
                (local.set $retp (i32.const 8))
                (local.set $ws-retp (i32.const 16))
                (local.set $ws (call $waitable-set.new))

                (if (i32.eq (i32.and (local.get 0) (i32.const 0xF)) (i32.const 2 (; RETURNED ;)))
                    (then (call $check (i32.load (local.get $retp))))
                    (else 
                        (call $waitable.join (i32.shr_u (local.get 0) (i32.const 4)) (local.get $ws))
                        (drop (call $waitable-set.wait (local.get $ws) (local.get $ws-retp)))
                        (call $check (i32.load offset=4 (local.get $ws-retp)))))
            )

            (func $run (export "run") (result i32)
                (local $retp i32)
                (local.set $retp (i32.const 8))
                (call $check (call $explicit-thread-calls-return-stackless))
                (call $check (call $explicit-thread-suspends-sync))
                (call $check (call $explicit-thread-suspends-stackful))
                (call $check (call $explicit-thread-suspends-stackless))
                (call $check (call $explicit-thread-yield-loops-sync))

                (call $check-async (call $explicit-thread-suspends-sync-async (local.get $retp)))
                (call $check-async (call $explicit-thread-yield-loops-sync-async (local.get $retp)))
                (call $check-async (call $explicit-thread-suspends-sync-async (local.get $retp)))
                (call $check-async (call $explicit-thread-yield-loops-sync-async (local.get $retp)))

                (i32.const 42)
            )
        )

        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))
        (core func $waitable.join (canon waitable.join))
        (core func $subtask.cancel (canon subtask.cancel async))
        (core func $thread.yield (canon thread.yield))
        ;; sync lowered
        (canon lower (func $explicit-thread-calls-return-stackful) (memory $memory "mem") (core func $explicit-thread-calls-return-stackful'))
        (canon lower (func $explicit-thread-calls-return-stackless) (memory $memory "mem") (core func $explicit-thread-calls-return-stackless'))
        (canon lower (func $explicit-thread-suspends-sync) (memory $memory "mem") (core func $explicit-thread-suspends-sync'))
        (canon lower (func $explicit-thread-suspends-stackful) (memory $memory "mem") (core func $explicit-thread-suspends-stackful'))
        (canon lower (func $explicit-thread-suspends-stackless) (memory $memory "mem") (core func $explicit-thread-suspends-stackless'))
        (canon lower (func $explicit-thread-yield-loops-sync) (memory $memory "mem") (core func $explicit-thread-yield-loops-sync'))
        (canon lower (func $explicit-thread-yield-loops-stackful) (memory $memory "mem") (core func $explicit-thread-yield-loops-stackful'))
        (canon lower (func $explicit-thread-yield-loops-stackless) (memory $memory "mem") (core func $explicit-thread-yield-loops-stackless'))
        ;; async lowered
        (canon lower (func $explicit-thread-calls-return-stackful) async (memory $memory "mem") (core func $explicit-thread-calls-return-stackful-async'))
        (canon lower (func $explicit-thread-calls-return-stackless) async (memory $memory "mem") (core func $explicit-thread-calls-return-stackless-async'))
        (canon lower (func $explicit-thread-suspends-sync) async (memory $memory "mem") (core func $explicit-thread-suspends-sync-async'))
        (canon lower (func $explicit-thread-suspends-stackful) async (memory $memory "mem") (core func $explicit-thread-suspends-stackful-async'))
        (canon lower (func $explicit-thread-suspends-stackless) async (memory $memory "mem") (core func $explicit-thread-suspends-stackless-async'))
        (canon lower (func $explicit-thread-yield-loops-sync) async (memory $memory "mem") (core func $explicit-thread-yield-loops-sync-async'))
        (canon lower (func $explicit-thread-yield-loops-stackful) async (memory $memory "mem") (core func $explicit-thread-yield-loops-stackful-async'))
        (canon lower (func $explicit-thread-yield-loops-stackless) async (memory $memory "mem") (core func $explicit-thread-yield-loops-stackless-async'))
        (core instance $dm (instantiate $DM (with "" (instance
            (export "mem" (memory $memory "mem"))
            (export "explicit-thread-calls-return-stackful" (func $explicit-thread-calls-return-stackful'))
            (export "explicit-thread-calls-return-stackless" (func $explicit-thread-calls-return-stackless'))
            (export "explicit-thread-suspends-sync" (func $explicit-thread-suspends-sync'))
            (export "explicit-thread-suspends-stackful" (func $explicit-thread-suspends-stackful'))
            (export "explicit-thread-suspends-stackless" (func $explicit-thread-suspends-stackless'))
            (export "explicit-thread-yield-loops-sync" (func $explicit-thread-yield-loops-sync'))
            (export "explicit-thread-yield-loops-stackful" (func $explicit-thread-yield-loops-stackful'))
            (export "explicit-thread-yield-loops-stackless" (func $explicit-thread-yield-loops-stackless'))
            (export "explicit-thread-calls-return-stackful-async" (func $explicit-thread-calls-return-stackful-async'))
            (export "explicit-thread-calls-return-stackless-async" (func $explicit-thread-calls-return-stackless-async'))
            (export "explicit-thread-suspends-sync-async" (func $explicit-thread-suspends-sync-async'))
            (export "explicit-thread-suspends-stackful-async" (func $explicit-thread-suspends-stackful-async'))
            (export "explicit-thread-suspends-stackless-async" (func $explicit-thread-suspends-stackless-async'))
            (export "explicit-thread-yield-loops-sync-async" (func $explicit-thread-yield-loops-sync-async'))
            (export "explicit-thread-yield-loops-stackful-async" (func $explicit-thread-yield-loops-stackful-async'))
            (export "explicit-thread-yield-loops-stackless-async" (func $explicit-thread-yield-loops-stackless-async'))
            (export "waitable.join" (func $waitable.join))
            (export "waitable-set.new" (func $waitable-set.new))
            (export "waitable-set.wait" (func $waitable-set.wait))
            (export "subtask.cancel" (func $subtask.cancel))
            (export "thread.yield" (func $thread.yield))
        ))))
        (func (export "run") (result u32) (canon lift (core func $dm "run")))
    )

    (instance $c (instantiate $C))
    (instance $d (instantiate $D
        (with "explicit-thread-calls-return-stackful" (func $c "explicit-thread-calls-return-stackful"))
        (with "explicit-thread-calls-return-stackless" (func $c "explicit-thread-calls-return-stackless"))
        (with "explicit-thread-suspends-sync" (func $c "explicit-thread-suspends-sync"))
        (with "explicit-thread-suspends-stackful" (func $c "explicit-thread-suspends-stackful"))
        (with "explicit-thread-suspends-stackless" (func $c "explicit-thread-suspends-stackless"))
        (with "explicit-thread-yield-loops-sync" (func $c "explicit-thread-yield-loops-sync"))
        (with "explicit-thread-yield-loops-stackful" (func $c "explicit-thread-yield-loops-stackful"))
        (with "explicit-thread-yield-loops-stackless" (func $c "explicit-thread-yield-loops-stackless"))
    ))
  (func (export "run") (alias export $d "run"))
)

(assert_return (invoke "run") (u32.const 42))
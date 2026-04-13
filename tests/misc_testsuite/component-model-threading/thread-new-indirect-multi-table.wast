;;! component_model_async = true
;;! component_model_threading = true
;;! reference_types = true

;; Regression test for a bug where the `ThreadNewIndirect` trampoline passed
;; `start_func_table_idx` and `start_func_ty_idx` in the wrong order to the
;; host `thread_new_indirect` libcall. When both indices happened to be 0 (the
;; common single-table case), the swap was invisible. This test uses two tables
;; and two `thread.new-indirect` canonicals so the indices differ, exposing the
;; bug: calling `thread.new-indirect` targeting the empty table should trap with
;; "uninitialized", not silently resolve a function from the wrong table.

(component
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref)
        (table (export "t1") 1 funcref))
    (core module $m
        (import "" "thread.new-indirect-dummy" (func $thread-new-indirect-dummy (param i32 i32) (result i32)))
        (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.index" (func $thread-index (result i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

        (func $thread-start (param i32))
        (export "thread-start" (func $thread-start))
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)

        (func $use-dummy (result i32)
            (call $thread-new-indirect-dummy (i32.const 0) (i32.const 0)))
        (export "use-dummy" (func $use-dummy))

        (func (export "run") (result i32)
            (call $thread-new-indirect (i32.const 0) (i32.const 42))))

    (core instance $libc (instantiate $libc))
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "__indirect_function_table" (core table $t0))
    (alias core export $libc "t1" (core table $t1))

    (core func $thread-new-indirect-t0
        (canon thread.new-indirect $start-func-ty (table $t0)))
    (core func $thread-new-indirect-t1
        (canon thread.new-indirect $start-func-ty (table $t1)))
    (core func $thread-index (canon thread.index))

    (core instance $i (instantiate $m
        (with "" (instance
            (export "thread.new-indirect-dummy" (func $thread-new-indirect-t0))
            (export "thread.new-indirect" (func $thread-new-indirect-t1))
            (export "thread.index" (func $thread-index))))
        (with "libc" (instance $libc))))

    (func (export "run") async (result u32) (canon lift (core func $i "run"))))

(assert_trap (invoke "run") "uninitialized")

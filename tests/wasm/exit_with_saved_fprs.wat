;;; This is a test case for https://github.com/bytecodealliance/wasmtime/issues/1967 
(module
    (type (func (param i32)))
    
    (import "wasi_snapshot_preview1" "proc_exit" (func (type 0)))
    (memory (export "memory") 0)
    
    (func $exit (param i32)
        local.get 0
        call 0
        unreachable
    )
    
    (func $do_something (param f64 f64 f64 f64 f64 f64 f64 f64)
        i32.const 0
        call $exit
        unreachable
    )
    
    (func $has_saved_fprs (export "_start")
        (local f64 f64 f64 f64 f64 f64 f64 f64)
        (local.set 0 (f64.const 1))
        (local.set 1 (f64.const 2))
        (local.set 2 (f64.const 3))
        (local.set 3 (f64.const 4))
        (local.set 4 (f64.const 5))
        (local.set 5 (f64.const 6))
        (local.set 6 (f64.const 7))
        (local.set 7 (f64.const 8))
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        local.get 4
        local.get 5
        local.get 6
        local.get 7
        call $do_something
        unreachable
    )
)

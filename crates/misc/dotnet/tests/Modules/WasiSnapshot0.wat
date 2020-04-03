(module
  (type $t0 (func (param i32 i32) (result i32)))
  (type $t1 (func (param i32 i32 i32 i32) (result i32)))
  (type $t2 (func (param i32) (result i32)))
  (type $t3 (func (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
  (import "wasi_unstable" "environ_sizes_get" (func $wasi_unstable.environ_sizes_get (type $t0)))
  (import "wasi_unstable" "environ_get" (func $wasi_unstable.environ_get (type $t0)))
  (import "wasi_unstable" "args_sizes_get" (func $wasi_unstable.args_sizes_get (type $t0)))
  (import "wasi_unstable" "args_get" (func $wasi_unstable.args_get (type $t0)))
  (import "wasi_unstable" "fd_write" (func $wasi_unstable.fd_write (type $t1)))
  (import "wasi_unstable" "fd_read" (func $wasi_unstable.fd_read (type $t1)))
  (import "wasi_unstable" "fd_close" (func $wasi_unstable.fd_close (type $t2)))
  (import "wasi_unstable" "path_open" (func $wasi_unstable.path_open (type $t3)))
  (memory $memory 1)
  (export "memory" (memory 0))
  (func $call_environ_sizes_get (type $t0) (param $p0 i32) (param $p1 i32) (result i32)
    local.get $p0
    local.get $p1
    call $wasi_unstable.environ_sizes_get)
  (func $call_environ_get (type $t0) (param $p0 i32) (param $p1 i32) (result i32)
    local.get $p0
    local.get $p1
    call $wasi_unstable.environ_get)
  (func $call_args_sizes_get (type $t0) (param $p0 i32) (param $p1 i32) (result i32)
    local.get $p0
    local.get $p1
    call $wasi_unstable.args_sizes_get)
  (func $call_args_get (type $t0) (param $p0 i32) (param $p1 i32) (result i32)
    local.get $p0
    local.get $p1
    call $wasi_unstable.args_get)
  (func $call_fd_write (type $t1) (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32) (result i32)
    local.get $p0
    local.get $p1
    local.get $p2
    local.get $p3
    call $wasi_unstable.fd_write)
  (func $call_fd_read (type $t1) (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32) (result i32)
    local.get $p0
    local.get $p1
    local.get $p2
    local.get $p3
    call $wasi_unstable.fd_read)
  (func $call_fd_close (type $t2) (param $p0 i32) (result i32)
    local.get $p0
    call $wasi_unstable.fd_close)
  (func $call_path_open (type $t3) (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32) (param $p4 i32) (param $p5 i64) (param $p6 i64) (param $p7 i32) (param $p8 i32) (result i32)
    local.get $p0
    local.get $p1
    local.get $p2
    local.get $p3
    local.get $p4
    local.get $p5
    local.get $p6
    local.get $p7
    local.get $p8
    call $wasi_unstable.path_open)
  (export "call_environ_sizes_get" (func $call_environ_sizes_get))
  (export "call_environ_get" (func $call_environ_get))
  (export "call_args_sizes_get" (func $call_args_sizes_get))
  (export "call_args_get" (func $call_args_get))
  (export "call_fd_write" (func $call_fd_write))
  (export "call_fd_read" (func $call_fd_read))
  (export "call_fd_close" (func $call_fd_close))
  (export "call_path_open" (func $call_path_open))
)

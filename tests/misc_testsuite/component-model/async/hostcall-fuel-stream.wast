;;! component_model_async = true

;; A `stream<u8>` write that completes a pending read is copied by the host, so
;; it's limited by the store's hostcall fuel. This runs with exactly the fuel a
;; 64 KiB copy needs and then traps with one byte less.

(component definition $stream
  (import "wasmtime" (instance $wasmtime
    (export "set-hostcall-fuel" (func (param "fuel" u64)))
  ))
  (core module $libc (memory (export "mem") 2))
  (core instance $libc (instantiate $libc))
  (type $s (stream u8))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read
    (canon stream.read $s async (memory (core memory $libc "mem"))))
  (core func $stream.write
    (canon stream.write $s async (memory (core memory $libc "mem"))))
  (core func $stream.cancel-read (canon stream.cancel-read $s))
  (core module $m
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read"
      (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write"
      (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.cancel-read"
      (func $stream.cancel-read (param i32) (result i32)))
    (func (export "run")
      (local $pair i64)
      (local $r i32)
      (local $w i32)
      (local.set $pair (call $stream.new))
      (local.set $r (i32.wrap_i64 (local.get $pair)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
      ;; BLOCKED
      (if (i32.ne (call $stream.read (local.get $r) (i32.const 0) (i32.const 65536))
                  (i32.const -1))
        (then unreachable))
      ;; (65536 << 4) | COMPLETED
      (if (i32.ne (call $stream.write (local.get $w) (i32.const 65536) (i32.const 65536))
                  (i32.const 0x100000))
        (then unreachable))
      ;; collect the read's completion so the task can exit
      (drop (call $stream.cancel-read (local.get $r)))))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.cancel-read" (func $stream.cancel-read))))))
  (core func $set-hostcall-fuel (canon lower (func $wasmtime "set-hostcall-fuel")))
  (core module $fuel
    (import "" "set" (func $set (param i64)))
    (func (export "set") (param i64) (call $set (local.get 0))))
  (core instance $fuel (instantiate $fuel
    (with "" (instance (export "set" (func $set-hostcall-fuel))))))
  (func (export "set-hostcall-fuel") (param "fuel" u64)
    (canon lift (core func $fuel "set")))
  (func (export "run") (canon lift (core func $i "run"))))

(component instance $i $stream)
(assert_return (invoke "set-hostcall-fuel" (u64.const 65536)))
(assert_return (invoke "run"))
(component instance $i $stream)
(assert_return (invoke "set-hostcall-fuel" (u64.const 65535)))
(assert_trap (invoke "run") "fuel allocated for hostcalls has been exhausted")

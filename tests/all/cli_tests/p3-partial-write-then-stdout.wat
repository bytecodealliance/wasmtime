;; Starts an 11-byte write to a `stream<u8>`, reads 4 of those bytes back
;; from the readable end, and then hands the readable end to
;; `wasi:cli/stdout#write-via-stream` while the write is still pending.
;;
;; The writer's event must account for all bytes consumed from its buffer:
;; either just the 4 read by the guest (in which case the remaining 7 bytes are
;; written again) or all 11 if the host consumed the rest. Either way
;; "456789\n" ends up on stdout.
(component
  (import "wasi:cli/types@0.3.0" (instance $types
    (type $e (enum "io" "illegal-byte-sequence" "pipe"))
    (export "error-code" (type (eq $e)))))
  (alias export $types "error-code" (type $ec))
  (import "wasi:cli/stdout@0.3.0" (instance $stdout
    (export "error-code" (type $ec2 (eq $ec)))
    (export "write-via-stream" (func
      (param "data" (stream u8)) (result (future (result (error $ec2))))))))

  (core module $libc
    (memory (export "mem") 1)
    (data (i32.const 0) "0123456789\n"))
  (core instance $libc (instantiate $libc))

  (type $s (stream u8))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.write (canon stream.write $s async (memory (core memory $libc "mem"))))
  (core func $stream.read (canon stream.read $s async (memory (core memory $libc "mem"))))
  (core func $stream.drop-writable (canon stream.drop-writable $s))
  (core func $write-via-stream (canon lower (func $stdout "write-via-stream")))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $task.return (canon task.return (result (result))))

  (core module $m
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))
    (import "" "write-via-stream" (func $write-via-stream (param i32) (result i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "join" (func $waitable.join (param i32 i32)))
    (import "" "task.return" (func $task.return (param i32)))
    (global $w (mut i32) (i32.const 0))
    (global $ws (mut i32) (i32.const 0))
    (global $rewrote (mut i32) (i32.const 0))

    (func (export "run") (result i32)
      (local $pair i64) (local $r i32)
      (local.set $pair (call $stream.new))
      (local.set $r (i32.wrap_i64 (local.get $pair)))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
      (if (i32.ne (call $stream.write (global.get $w) (i32.const 0) (i32.const 11))
                  (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (if (i32.ne (call $stream.read (local.get $r) (i32.const 100) (i32.const 4))
                  (i32.const 0x40 (; COMPLETED(4) ;)))
        (then unreachable))
      (drop (call $write-via-stream (local.get $r)))
      (global.set $ws (call $waitable-set.new))
      (call $waitable.join (global.get $w) (global.get $ws))
      (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4))))

    (func $finish (result i32)
      (call $waitable.join (global.get $w) (i32.const 0))
      (call $stream.drop-writable (global.get $w))
      (call $task.return (i32.const 0))
      (i32.const 0 (; EXIT ;)))

    (func (export "cb") (param $ev i32) (param $idx i32) (param $payload i32) (result i32)
      (local $ret i32)
      (if (i32.ne (local.get $ev) (i32.const 3 (; STREAM_WRITE ;))) (then unreachable))

      ;; if COMPLETED(7), then the re-write succeeded
      (if (global.get $rewrote)
        (then
          (if (i32.ne (local.get $payload) (i32.const 0x70)) (then unreachable))
          (return (call $finish))))

      ;; if COMPLETED(11), then the host consumed the whole buffer in one go.
      (if (i32.eq (local.get $payload) (i32.const 0xb0))
        (then (return (call $finish))))

      ;; if COMPLETED(4), then write the rest
      (if (i32.ne (local.get $payload) (i32.const 0x40)) (then unreachable))
      (global.set $rewrote (i32.const 1))
      (local.set $ret (call $stream.write (global.get $w) (i32.const 4) (i32.const 7)))
      (if (i32.eq (local.get $ret) (i32.const 0x70))
        (then (return (call $finish))))
      (if (i32.ne (local.get $ret) (i32.const -1)) (then unreachable))
      (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4))))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "stream.new" (func $stream.new))
    (export "stream.write" (func $stream.write))
    (export "stream.read" (func $stream.read))
    (export "stream.drop-writable" (func $stream.drop-writable))
    (export "write-via-stream" (func $write-via-stream))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "join" (func $waitable.join))
    (export "task.return" (func $task.return))))))
  (func $run async (result (result))
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  (instance $run (export "run" (func $run)))
  (export "wasi:cli/run@0.3.0" (instance $run))
)

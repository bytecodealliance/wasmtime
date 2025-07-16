;;! component_model_async = true

;; This test contains two components $C and $D that test cancelling reads
;; and writes in the presence and absence of partial reads/writes.
;;
;; $C exports a function 'start-stream' that creates and holds onto a writable
;;   stream in the global $sw as well as various operations that operate on $sw.
;; $D calls $C.start-stream to get the readable end and then drives the test.
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/add-tests/test/concurrency/cancel-stream.wast)
(component
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "stream.cancel-write" (func $stream.cancel-write (param i32) (result i32)))
      (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))

      (global $sw (mut i32) (i32.const 0))

      (func $start-stream (export "start-stream") (result i32)
        ;; create a new stream, return the readable end to the caller
        (local $ret64 i64)
        (local.set $ret64 (call $stream.new))
        (global.set $sw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (i32.wrap_i64 (local.get $ret64))
      )

      (func $write4 (export "write4")
        ;; write 6 bytes into the stream, expecting to rendezvous with a stream.read
        (local $ret i32)
        (i32.store (i32.const 8) (i32.const 0xabcd))
        (local.set $ret (call $stream.write (global.get $sw) (i32.const 8) (i32.const 4)))
        (if (i32.ne (i32.const 0x40 (; COMPLETED=0 | (4<<4) ;)) (local.get $ret))
          (then unreachable))
      )

      (func $write4-and-drop (export "write4-and-drop")
        (call $write4)
        (call $stream.drop-writable (global.get $sw))
      )

      (func $start-blocking-write (export "start-blocking-write")
        (local $ret i32)

        ;; prepare the write buffer
        (i64.store (i32.const 8) (i64.const 0x123456789abcdef))

        ;; start one blocking write and immediately cancel it
        (local.set $ret (call $stream.write (global.get $sw) (i32.const 8) (i32.const 8)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))
        (local.set $ret (call $stream.cancel-write (global.get $sw)))
        (if (i32.ne (i32.const 0x2 (; CANCELLED ;)) (local.get $ret))
          (then unreachable))

        ;; start a second blockign write and leave it pending
        (local.set $ret (call $stream.write (global.get $sw) (i32.const 8) (i32.const 8)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))
      )

      (func $cancel-after-read4 (export "cancel-after-read4")
        (local $ret i32)
        (local.set $ret (call $stream.cancel-write (global.get $sw)))
        (if (i32.ne (i32.const 0x42 (; CANCELLED=2 | (4<<4) ;)) (local.get $ret))
          (then unreachable))
      )
    )
    (type $ST (stream u8))
    (canon task.return (result u32) (core func $task.return))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.write $ST async (memory $memory "mem") (core func $stream.write))
    (canon stream.cancel-write $ST (core func $stream.cancel-write))
    (canon stream.drop-writable $ST (core func $stream.drop-writable))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "task.return" (func $task.return))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))
      (export "stream.cancel-write" (func $stream.cancel-write))
      (export "stream.drop-writable" (func $stream.drop-writable))
    ))))
    (func (export "start-stream") (result (stream u8)) (canon lift (core func $cm "start-stream")))
    (func (export "write4") (canon lift (core func $cm "write4")))
    (func (export "write4-and-drop") (canon lift (core func $cm "write4-and-drop")))
    (func (export "start-blocking-write") (canon lift (core func $cm "start-blocking-write")))
    (func (export "cancel-after-read4") (canon lift (core func $cm "cancel-after-read4")))
  )

  (component $D
    (import "c" (instance $c
      (export "start-stream" (func (result (stream u8))))
      (export "write4" (func))
      (export "write4-and-drop" (func))
      (export "start-blocking-write" (func))
      (export "cancel-after-read4" (func))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $DM
      (import "" "mem" (memory 1))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
      (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
      (import "" "start-stream" (func $start-stream (result i32)))
      (import "" "write4" (func $write4))
      (import "" "write4-and-drop" (func $write4-and-drop))
      (import "" "start-blocking-write" (func $start-blocking-write))
      (import "" "cancel-after-read4" (func $cancel-after-read4))

      (func $run (export "run") (result i32)
        (local $ret i32)
        (local $sr i32)

        ;; call 'start-stream' to get the stream we'll be working with
        (local.set $sr (call $start-stream))
        (if (i32.ne (i32.const 1) (local.get $sr))
          (then unreachable))

        ;; start read that will block
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 8) (i32.const 100)))
        (if (i32.ne (i32.const -1 (; BLOCKED;)) (local.get $ret))
          (then unreachable))

        ;; cancelling it will finish without anything having been written
        (local.set $ret (call $stream.cancel-read (local.get $sr)))
        (if (i32.ne (i32.const 0x2 (; CANCELLED ;)) (local.get $ret))
          (then unreachable))

        ;; read, block, call $C to write 4 bytes into the buffer,
        ;; then cancel, which should show "4+cancelled"
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 8) (i32.const 100)))
        (if (i32.ne (i32.const -1 (; BLOCKED;)) (local.get $ret))
          (then unreachable))
        (call $write4)
        (local.set $ret (call $stream.cancel-read (local.get $sr)))
        (if (i32.ne (i32.const 0x42 (; CANCELLED=2 | (4<<4) ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 0xabcd) (i32.load (i32.const 8)))
          (then unreachable))

        ;; read, block, call $C to write 4 bytes into the buffer and drop,
        ;; then cancel, which should show "4+dropped"
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 8) (i32.const 100)))
        (if (i32.ne (i32.const -1 (; BLOCKED;)) (local.get $ret))
          (then unreachable))
        (call $write4-and-drop)
        (local.set $ret (call $stream.cancel-read (local.get $sr)))
        (if (i32.ne (i32.const 0x41 (; DROPPED=1 | (4<<4) ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 0xabcd) (i32.load (i32.const 8)))
          (then unreachable))
        (call $stream.drop-readable (local.get $sr))

        ;; get a new $sr
        (local.set $sr (call $start-stream))
        (if (i32.ne (i32.const 1) (local.get $sr))
          (then unreachable))

        ;; start outstanding write in $C, read 4 of it, then call back into $C
        ;; which will cancel and see 4 written.
        (call $start-blocking-write)
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 8) (i32.const 4)))
        (if (i32.ne (i32.const 0x40 (; COMPLETED=0 | (4<<4) ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 0x89abcdef) (i32.load (i32.const 8)))
          (then unreachable))
        (call $cancel-after-read4)

        ;; return 42 to the top-level assert_return
        (i32.const 42)
      )
    )
    (type $ST (stream u8))
    (canon stream.read $ST async (memory $memory "mem") (core func $stream.read))
    (canon stream.cancel-read $ST (core func $stream.cancel-read))
    (canon stream.drop-readable $ST (core func $stream.drop-readable))
    (canon lower (func $c "start-stream") (core func $start-stream'))
    (canon lower (func $c "write4") (core func $write4'))
    (canon lower (func $c "write4-and-drop") (core func $write4-and-drop'))
    (canon lower (func $c "start-blocking-write") (core func $start-blocking-write'))
    (canon lower (func $c "cancel-after-read4") (core func $cancel-after-read4'))
    (core instance $dm (instantiate $DM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "stream.read" (func $stream.read))
      (export "stream.cancel-read" (func $stream.cancel-read))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "start-stream" (func $start-stream'))
      (export "write4" (func $write4'))
      (export "write4-and-drop" (func $write4-and-drop'))
      (export "start-blocking-write" (func $start-blocking-write'))
      (export "cancel-after-read4" (func $cancel-after-read4'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $dm "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 42))

;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! reference_types = true

;; This test has two components $C and $D, where $D imports and calls $C.
;; $C contains utility functions used by $D to create futures/streams,
;; write to them and close them. $D uses these utility functions to test for
;; all the cases where, once a future/stream is "done", further uses of the
;; future/stream trap.
;;
;; $D exports a list of functions, one for each case of trapping. Since traps
;; take out their containing instance, a fresh instance of $Tester is created
;; for each call to a $D export.
;;
;; When testing traps involving the readable end, the exports of $D take a
;; "bool" parameter that toggles whether the trap is triggered by
;; {stream,future}.{read,write} or by lifting, and the top-level commands
;; pass 'false' and 'true'.
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/fix-future/test/async/trap-if-done.wast)
(component definition $Tester
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "future.write-sync" (func $future.write-sync (param i32 i32) (result i32)))
      (import "" "future.drop-writable" (func $future.drop-writable (param i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "stream.write-sync" (func $stream.write-sync (param i32 i32 i32) (result i32)))
      (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))

      (global $writable-end (mut i32) (i32.const 0))
      (global $ws (mut i32) (i32.const 0))

      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      (func $start-future (export "start-future") (result i32)
        ;; create a new future, return the readable end to the caller
        (local $ret64 i64)
        (local.set $ret64 (call $future.new))
        (global.set $writable-end (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (call $waitable.join (global.get $writable-end) (global.get $ws) )
        (i32.wrap_i64 (local.get $ret64))
      )
      (func $future-write (export "future-write") (result i32)
        ;; the caller will assert what they expect the return value to be
        (i32.store (i32.const 16) (i32.const 42))
        (call $future.write (global.get $writable-end) (i32.const 16))
      )
      (func $future-write-sync (export "future-write-sync") (result i32)
        ;; the caller will assert what they expect the return value to be
        (i32.store (i32.const 16) (i32.const 42))
        (call $future.write-sync (global.get $writable-end) (i32.const 16))
      )
      (func $acknowledge-future-write (export "acknowledge-future-write") (result i32)
        ;; confirm we got a FUTURE_WRITE $writable-end event
        (local $ret i32)
        (local.set $ret (call $waitable-set.wait (global.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 5 (; FUTURE_WRITE ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (global.get $writable-end) (i32.load (i32.const 0)))
          (then unreachable))
        (i32.load (i32.const 4))
      )
      (func $future-drop-writable (export "future-drop-writable")
        ;; maybe boom
        (call $future.drop-writable (global.get $writable-end))
      )

      (func $start-stream (export "start-stream") (result i32)
        ;; create a new stream, return the readable end to the caller
        (local $ret64 i64)
        (local.set $ret64 (call $stream.new))
        (global.set $writable-end (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (call $waitable.join (global.get $writable-end) (global.get $ws) )
        (i32.wrap_i64 (local.get $ret64))
      )
      (func $stream-write (export "stream-write") (result i32)
        ;; the caller will assert what they expect the return value to be
        (i32.store (i32.const 16) (i32.const 42))
        (call $stream.write (global.get $writable-end) (i32.const 16) (i32.const 1))
      )
      (func $stream-write-sync (export "stream-write-sync") (result i32)
        ;; the caller will assert what they expect the return value to be
        (i32.store (i32.const 16) (i32.const 42))
        (call $stream.write-sync (global.get $writable-end) (i32.const 16) (i32.const 1))
      )
      (func $acknowledge-stream-write (export "acknowledge-stream-write")
        ;; confirm we got a STREAM_WRITE $writable-end COMPLETED event
        (local $ret i32)
        (local.set $ret (call $waitable-set.wait (global.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 3 (; STREAM_WRITE ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (global.get $writable-end) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 0x11 (; DROPPED=1 | (1<<4) ;)) (i32.load (i32.const 4)))
          (then unreachable))
      )
      (func $stream-drop-writable (export "stream-drop-writable")
        ;; maybe boom
        (call $stream.drop-writable (global.get $writable-end))
      )
    )
    (type $FT (future u8))
    (type $ST (stream u8))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon future.new $FT (core func $future.new))
    (canon future.write $FT async (memory $memory "mem") (core func $future.write))
    (canon future.write $FT (memory $memory "mem") (core func $future.write-sync))
    (canon future.drop-writable $FT (core func $future.drop-writable))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.write $ST async (memory $memory "mem") (core func $stream.write))
    (canon stream.write $ST (memory $memory "mem") (core func $stream.write-sync))
    (canon stream.drop-writable $ST (core func $stream.drop-writable))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "future.write-sync" (func $future.write-sync))
      (export "future.drop-writable" (func $future.drop-writable))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))
      (export "stream.write-sync" (func $stream.write-sync))
      (export "stream.drop-writable" (func $stream.drop-writable))
    ))))
    (func (export "start-future") (result (future u8)) (canon lift (core func $cm "start-future")))
    (func (export "future-write") (result u32) (canon lift (core func $cm "future-write")))
    (func (export "future-write-sync") async (result u32) (canon lift (core func $cm "future-write-sync")))
    (func (export "acknowledge-future-write") async (result u32) (canon lift (core func $cm "acknowledge-future-write")))
    (func (export "future-drop-writable") (canon lift (core func $cm "future-drop-writable")))
    (func (export "start-stream") (result (stream u8)) (canon lift (core func $cm "start-stream")))
    (func (export "stream-write") (result u32) (canon lift (core func $cm "stream-write")))
    (func (export "stream-write-sync") async (result u32) (canon lift (core func $cm "stream-write-sync")))
    (func (export "acknowledge-stream-write") async (canon lift (core func $cm "acknowledge-stream-write")))
    (func (export "stream-drop-writable") (canon lift (core func $cm "stream-drop-writable")))
  )
  (component $D
    (import "c" (instance $c
      (export "start-future" (func (result (future u8))))
      (export "future-write" (func (result u32)))
      (export "future-write-sync" (func async (result u32)))
      (export "acknowledge-future-write" (func async (result u32)))
      (export "future-drop-writable" (func))
      (export "start-stream" (func (result (stream u8))))
      (export "stream-write" (func (result u32)))
      (export "stream-write-sync" (func async (result u32)))
      (export "acknowledge-stream-write" (func async))
      (export "stream-drop-writable" (func))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $Core
      (import "" "mem" (memory 1))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "future.read-sync" (func $future.read-sync (param i32 i32) (result i32)))
      (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "stream.read-sync" (func $stream.read-sync (param i32 i32 i32) (result i32)))
      (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
      (import "" "start-future" (func $start-future (result i32)))
      (import "" "future-write" (func $future-write (result i32)))
      (import "" "future-write-sync.sync" (func $future-write-sync.sync (result i32)))
      (import "" "future-write-sync.async" (func $future-write-sync.async (param i32) (result i32)))
      (import "" "acknowledge-future-write" (func $acknowledge-future-write (result i32)))
      (import "" "future-drop-writable" (func $future-drop-writable))
      (import "" "start-stream" (func $start-stream (result i32)))
      (import "" "stream-write" (func $stream-write (result i32)))
      (import "" "stream-write-sync" (func $stream-write-sync (result i32)))
      (import "" "acknowledge-stream-write" (func $acknowledge-stream-write))
      (import "" "stream-drop-writable" (func $stream-drop-writable))

      (func $future-write-expect-trap (param $which i32)
        (if (i32.eq (local.get $which) (i32.const 0))
          (then (drop (call $future-write))))
        (if (i32.eq (local.get $which) (i32.const 1))
          (then (drop (call $future-write-sync.sync))))
        unreachable
      )

      (func $future-read-expect-trap (param $fr i32) (param $which i32) (result i32)
        ;; calling future.read again should then trap
        (if (i32.eq (local.get $which) (i32.const 0))
          (then (drop (call $future.read (local.get $fr) (i32.const 16)))))
        (if (i32.eq (local.get $which) (i32.const 1))
          (then (drop (call $future.read-sync (local.get $fr) (i32.const 16)))))

        ;; lifting the future by returning it should also trap
        (if (i32.eq (local.get $which) (i32.const 2))
          (then (return (local.get $fr))))

        unreachable
      )

      (func $stream-write-expect-trap (param $which i32)
        (if (i32.eq (local.get $which) (i32.const 0))
          (then (drop (call $stream-write))))
        (if (i32.eq (local.get $which) (i32.const 1))
          (then (drop (call $stream-write-sync))))
        unreachable
      )

      (func $stream-read-expect-trap (param $sr i32) (param $which i32) (result i32)
        ;; calling stream.read again should then trap
        (if (i32.eq (local.get $which) (i32.const 0))
          (then (drop (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))))
        (if (i32.eq (local.get $which) (i32.const 1))
          (then (drop (call $stream.read-sync (local.get $sr) (i32.const 16) (i32.const 100)))))

        ;; lifting the stream by returning it should also trap
        (if (i32.eq (local.get $which) (i32.const 2))
          (then (return (local.get $sr))))

        unreachable
      )

      (func $future-eager-write (param $which i32) (result i32)
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; start a read on our end so the next write will succeed
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; calling future.write in $C should succeed eagerly
        (if (local.get $which)
          (then (local.set $ret (call $future-write-sync.sync)))
          (else (local.set $ret (call $future-write))))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        local.get $fr
      )

      (func $future-async-write (param $which i32) (result i32)
        (local $ret i32)
        (local $fr i32)
        (local $ws i32)
        (local $subtask i32)
        (local.set $fr (call $start-future))

        ;; calling future.write in $C should block
        local.get $which
        if
          (local.set $ret (call $future-write-sync.async (i32.const 8)))
          (if (i32.ne (i32.const 1 (; SUBTASK_STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
            (then unreachable))
          (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
        else
          (local.set $ret (call $future-write))
          (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
            (then unreachable))
        end

        ;; our future.read should then succeed eagerly
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        local.get $which
        if
          ;; wait to see that our blocked future.read COMPLETED, producing '42'
          (local.set $ws (call $waitable-set.new))
          (call $waitable.join (local.get $subtask) (local.get $ws))
          (local.set $ret (call $waitable-set.wait (local.get $ws) (i32.const 0)))
          (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $ret))
            (then unreachable))
          (if (i32.ne (local.get $subtask) (i32.load (i32.const 0)))
            (then unreachable))
          (if (i32.ne (i32.const 2 (; RETURNED ;)) (i32.load (i32.const 4)))
            (then unreachable))
          (if (i32.ne (i32.const 0 (; COMPLETED ;)) (i32.load (i32.const 8)))
            (then unreachable))
        else
          ;; let $C see the write completed so the future is 'done'
          (if (i32.ne (i32.const 0 (; COMPLETED ;) (call $acknowledge-future-write)))
            (then unreachable))
        end

        local.get $fr
      )

      (func $future-async-drop (param $which i32)
        (local $ret i32)
        (local $fr i32)
        (local $ws i32)
        (local $subtask i32)
        (local.set $fr (call $start-future))

        ;; calling future.write in $C should block
        local.get $which
        if
          (local.set $ret (call $future-write-sync.async (i32.const 8)))
          (if (i32.ne (i32.const 1 (; SUBTASK_STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
            (then unreachable))
          (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
        else
          (local.set $ret (call $future-write))
          (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
            (then unreachable))
        end

        (call $future.drop-readable (local.get $fr))

        local.get $which
        if
          ;; wait to see that our blocked future.read COMPLETED, producing '42'
          (local.set $ws (call $waitable-set.new))
          (call $waitable.join (local.get $subtask) (local.get $ws))
          (local.set $ret (call $waitable-set.wait (local.get $ws) (i32.const 0)))
          (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $ret))
            (then unreachable))
          (if (i32.ne (local.get $subtask) (i32.load (i32.const 0)))
            (then unreachable))
          (if (i32.ne (i32.const 2 (; RETURNED ;)) (i32.load (i32.const 4)))
            (then unreachable))
          (if (i32.ne (i32.const 1 (; DROPPED ;)) (i32.load (i32.const 8)))
            (then unreachable))
        else
          ;; let $C see the write completed so the future is 'done'
          (if (i32.ne (i32.const 1 (; DROPPED ;) (call $acknowledge-future-write)))
            (then unreachable))
        end
      )

      (func $trap-after-future-eager-write (export "trap-after-future-eager-write")
        (param $flags i32)
        (drop (call $future-eager-write (i32.and (local.get $flags) (i32.const 2))))
        ;; calling future.write in $C now should trap
        (call $future-write-expect-trap (i32.and (local.get $flags) (i32.const 1)))
      )
      (func $trap-after-future-async-write (export "trap-after-future-async-write")
        (param $flags i32)
        (local $fr i32)

        (local.set $fr (call $future-async-write (i32.and (local.get $flags) (i32.const 2))))

        ;; trying to call future.write again in $C should trap
        (call $future-write-expect-trap (i32.and (local.get $flags) (i32.const 1)))
      )
      (func $trap-after-future-reader-dropped (export "trap-after-future-reader-dropped")
        (param $flags i32)
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; drop our readable end before writer can write
        (call $future.drop-readable (local.get $fr))

        ;; let $C try to future.write and find out we DROPPED
        (if (i32.and (local.get $flags) (i32.const 2))
          (then (local.set $ret (call $future-write-sync.sync)))
          (else (local.set $ret (call $future-write))))
        (if (i32.ne (i32.const 1 (; DROPPED ;)) (local.get $ret))
          (then unreachable))

        ;; trying to call future.write again in $C should trap
        (call $future-write-expect-trap (i32.and (local.get $flags) (i32.const 1)))
      )
      (func $trap-after-future-reader-async-dropped (export "trap-after-future-reader-async-dropped")
        (param $flags i32)
        (call $future-async-drop (i32.and (local.get $flags) (i32.const 2)))
        ;; trying to call future.write again in $C should trap
        (call $future-write-expect-trap (i32.and (local.get $flags) (i32.const 1)))
      )
      (func $trap-after-future-eager-read (export "trap-after-future-eager-read") (param $flags i32) (result i32)
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $future-async-write (i32.and (local.get $flags) (i32.const 0x10))))
        (call $future-read-expect-trap (local.get $fr) (i32.and (local.get $flags) (i32.const 0x0f)))
      )
      (func $trap-after-future-async-read (export "trap-after-future-async-read") (param $flags i32) (result i32)
        (local $ret i32) (local $ws i32)
        (local $fr i32)

        ;; read first, so it blocks
        (local.set $fr (call $future-eager-write (i32.and (local.get $flags) (i32.const 0x10))))

        ;; wait to see that our blocked future.read COMPLETED, producing '42'
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $fr) (local.get $ws))
        (local.set $ret (call $waitable-set.wait (local.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 4 (; FUTURE_READ ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (local.get $fr) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (i32.load (i32.const 4)))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load (i32.const 16)))
          (then unreachable))

        (call $future-read-expect-trap (local.get $fr) (i32.and (local.get $flags) (i32.const 0x03)))
      )
      (func $trap-after-stream-reader-eager-dropped (export "trap-after-stream-reader-eager-dropped")
        (param $which i32)
        (local $ret i32)
        (local $sr i32)
        (local.set $sr (call $start-stream))

        ;; drop our readable end before writer can write
        (call $stream.drop-readable (local.get $sr))

        ;; let $C try to stream.write and find out we DROPPED
        (local.set $ret (call $stream-write))
        (if (i32.ne (i32.const 1 (; DROPPED ;)) (local.get $ret))
          (then unreachable))

        ;; trying to call stream.write again in $C should trap
        (call $stream-write-expect-trap (local.get $which))
      )
      (func $trap-after-stream-reader-async-dropped (export "trap-after-stream-reader-async-dropped")
        (param $which i32)
        (local $ret i32)
        (local $sr i32)
        (local.set $sr (call $start-stream))

        ;; calling stream.write in $C should block
        (local.set $ret (call $stream-write))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; our stream.read should then succeed eagerly
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))
        (if (i32.ne (i32.const 0x10 (; COMPLETED=0 | (1<<4) ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        ;; then drop our readable end
        (call $stream.drop-readable (local.get $sr))

        ;; let $C see that it's stream.write COMPLETED and wrote 1 elem
        (call $acknowledge-stream-write)

        ;; now calling stream.write again in $C will trap
        (call $stream-write-expect-trap (local.get $which))
      )
      (func $trap-after-stream-writer-eager-dropped (export "trap-after-stream-writer-eager-dropped") (param $which i32) (result i32)
        (local $ret i32)
        (local $sr i32)
        (local.set $sr (call $start-stream))

        ;; immediately drop the writable end
        (call $stream-drop-writable)

        ;; calling stream.read will see that the writer dropped
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))
        (if (i32.ne (i32.const 0x01 (; DROPPED=1 | (0<<4) ;)) (local.get $ret))
          (then unreachable))

        (call $stream-read-expect-trap (local.get $sr) (local.get $which))
      )
      (func $trap-after-stream-writer-async-dropped (export "trap-after-stream-writer-async-dropped") (param $which i32) (result i32)
        (local $ret i32) (local $ws i32)
        (local $sr i32)
        (local.set $sr (call $start-stream))

        ;; start a read on our end first which will block
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; drop the writable end before writing anything
        (call $stream-drop-writable)

        ;; wait to see that our blocked stream.read was DROPPED
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $sr) (local.get $ws))
        (local.set $ret (call $waitable-set.wait (local.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 2 (; STREAM_READ ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (local.get $sr) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 0x01 (; DROPPED=1 | (0<<4) ;)) (i32.load (i32.const 4)))
          (then unreachable))

        (call $stream-read-expect-trap (local.get $sr) (local.get $which))
      )
    )
    (type $FT (future u8))
    (type $ST (stream u8))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon future.new $FT (core func $future.new))
    (canon future.read $FT async (memory $memory "mem") (core func $future.read))
    (canon future.read $FT (memory $memory "mem") (core func $future.read-sync))
    (canon future.drop-readable $FT (core func $future.drop-readable))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.read $ST async (memory $memory "mem") (core func $stream.read))
    (canon stream.read $ST (memory $memory "mem") (core func $stream.read-sync))
    (canon stream.drop-readable $ST (core func $stream.drop-readable))
    (canon lower (func $c "start-future") (core func $start-future'))
    (canon lower (func $c "future-write") (core func $future-write'))
    (canon lower (func $c "future-write-sync") (core func $future-write-sync.sync))
    (canon lower (func $c "future-write-sync") async (memory $memory "mem") (core func $future-write-sync.async))
    (canon lower (func $c "acknowledge-future-write") (core func $acknowledge-future-write'))
    (canon lower (func $c "future-drop-writable") (core func $future-drop-writable'))
    (canon lower (func $c "start-stream") (core func $start-stream'))
    (canon lower (func $c "stream-write") (core func $stream-write'))
    (canon lower (func $c "stream-write-sync") (core func $stream-write-sync'))
    (canon lower (func $c "acknowledge-stream-write") (core func $acknowledge-stream-write'))
    (canon lower (func $c "stream-drop-writable") (core func $stream-drop-writable'))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.read-sync" (func $future.read-sync))
      (export "future.drop-readable" (func $future.drop-readable))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.read-sync" (func $stream.read-sync))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "start-future" (func $start-future'))
      (export "future-write" (func $future-write'))
      (export "future-write-sync.sync" (func $future-write-sync.sync))
      (export "future-write-sync.async" (func $future-write-sync.async))
      (export "acknowledge-future-write" (func $acknowledge-future-write'))
      (export "future-drop-writable" (func $future-drop-writable'))
      (export "start-stream" (func $start-stream'))
      (export "stream-write" (func $stream-write'))
      (export "stream-write-sync" (func $stream-write-sync'))
      (export "acknowledge-stream-write" (func $acknowledge-stream-write'))
      (export "stream-drop-writable" (func $stream-drop-writable'))
    ))))
    (func (export "trap-after-future-eager-write") async (param "flags" u8) (canon lift (core func $core "trap-after-future-eager-write")))
    (func (export "trap-after-future-async-write") async (param "which" u8) (canon lift (core func $core "trap-after-future-async-write")))
    (func (export "trap-after-future-reader-dropped") async (param "which" u8) (canon lift (core func $core "trap-after-future-reader-dropped")))
    (func (export "trap-after-future-reader-async-dropped") async (param "which" u8) (canon lift (core func $core "trap-after-future-reader-async-dropped")))
    (func (export "trap-after-future-eager-read") async (param "which" u8) (result $FT) (canon lift (core func $core "trap-after-future-eager-read")))
    (func (export "trap-after-future-async-read") async (param "which" u8) (result $FT) (canon lift (core func $core "trap-after-future-async-read")))
    (func (export "trap-after-stream-reader-eager-dropped") async (param "which" u8) (canon lift (core func $core "trap-after-stream-reader-eager-dropped")))
    (func (export "trap-after-stream-reader-async-dropped") async (param "which" u8) (canon lift (core func $core "trap-after-stream-reader-async-dropped")))
    (func (export "trap-after-stream-writer-eager-dropped") async (param "which" u8) (result $ST) (canon lift (core func $core "trap-after-stream-writer-eager-dropped")))
    (func (export "trap-after-stream-writer-async-dropped") async (param "which" u8) (result $ST) (canon lift (core func $core "trap-after-stream-writer-async-dropped")))
  )
  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "trap-after-future-eager-write") (alias export $d "trap-after-future-eager-write"))
  (func (export "trap-after-future-async-write") (alias export $d "trap-after-future-async-write"))
  (func (export "trap-after-future-reader-dropped") (alias export $d "trap-after-future-reader-dropped"))
  (func (export "trap-after-future-reader-async-dropped") (alias export $d "trap-after-future-reader-async-dropped"))
  (func (export "trap-after-future-eager-read") (alias export $d "trap-after-future-eager-read"))
  (func (export "trap-after-future-async-read") (alias export $d "trap-after-future-async-read"))
  (func (export "trap-after-stream-reader-eager-dropped") (alias export $d "trap-after-stream-reader-eager-dropped"))
  (func (export "trap-after-stream-reader-async-dropped") (alias export $d "trap-after-stream-reader-async-dropped"))
  (func (export "trap-after-stream-writer-eager-dropped") (alias export $d "trap-after-stream-writer-eager-dropped"))
  (func (export "trap-after-stream-writer-async-dropped") (alias export $d "trap-after-stream-writer-async-dropped"))
)

(component instance $i1.0 $Tester)
(assert_trap (invoke "trap-after-future-eager-write" (u8.const 0)) "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i1.1 $Tester)
(assert_trap (invoke "trap-after-future-eager-write" (u8.const 1)) "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i1.2 $Tester)
(assert_trap (invoke "trap-after-future-eager-write" (u8.const 2)) "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i1.3 $Tester)
(assert_trap (invoke "trap-after-future-eager-write" (u8.const 3)) "cannot write to future after previous write succeeded or readable end dropped")

(component instance $i2.0 $Tester)
(assert_trap (invoke "trap-after-future-async-write" (u8.const 0)) "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i2.1 $Tester)
(assert_trap (invoke "trap-after-future-async-write" (u8.const 1)) "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i2.2 $Tester)
(assert_trap (invoke "trap-after-future-async-write" (u8.const 2)) "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i2.3 $Tester)
(assert_trap (invoke "trap-after-future-async-write" (u8.const 3)) "cannot write to future after previous write succeeded or readable end dropped")

(component instance $i3.0 $Tester)
(assert_trap (invoke "trap-after-future-reader-dropped" (u8.const 0)) "cannot write after being notified that the readable end dropped")
(component instance $i3.1 $Tester)
(assert_trap (invoke "trap-after-future-reader-dropped" (u8.const 1)) "cannot write after being notified that the readable end dropped")
(component instance $i3.2 $Tester)
(assert_trap (invoke "trap-after-future-reader-dropped" (u8.const 2)) "cannot write after being notified that the readable end dropped")
(component instance $i3.3 $Tester)
(assert_trap (invoke "trap-after-future-reader-dropped" (u8.const 3)) "cannot write after being notified that the readable end dropped")

(component instance $i $Tester)
(assert_trap (invoke "trap-after-future-reader-async-dropped" (u8.const 0)) "cannot write after being notified that the readable end dropped")
(component instance $i $Tester)
(assert_trap (invoke "trap-after-future-reader-async-dropped" (u8.const 1)) "cannot write after being notified that the readable end dropped")
(component instance $i $Tester)
(assert_trap (invoke "trap-after-future-reader-async-dropped" (u8.const 2)) "cannot write after being notified that the readable end dropped")
(component instance $i $Tester)
(assert_trap (invoke "trap-after-future-reader-async-dropped" (u8.const 3)) "cannot write after being notified that the readable end dropped")

(component instance $i4.0 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (u8.const 0x00)) "cannot read from future after previous read succeeded")
(component instance $i4.1 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (u8.const 0x01)) "cannot read from future after previous read succeeded")
(component instance $i4.2 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (u8.const 0x02)) "cannot lift future after previous read succeeded")
(component instance $i4.0 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (u8.const 0x10)) "cannot read from future after previous read succeeded")
(component instance $i4.1 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (u8.const 0x11)) "cannot read from future after previous read succeeded")
(component instance $i4.2 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (u8.const 0x12)) "cannot lift future after previous read succeeded")

(component instance $i5.0 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (u8.const 0x00)) "cannot read from future after previous read succeeded")
(component instance $i5.1 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (u8.const 0x01)) "cannot read from future after previous read succeeded")
(component instance $i5.2 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (u8.const 0x02)) "cannot lift future after previous read succeeded")
(component instance $i5.0 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (u8.const 0x10)) "cannot read from future after previous read succeeded")
(component instance $i5.1 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (u8.const 0x11)) "cannot read from future after previous read succeeded")
(component instance $i5.2 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (u8.const 0x12)) "cannot lift future after previous read succeeded")

(component instance $i6.0 $Tester)
(assert_trap (invoke "trap-after-stream-reader-eager-dropped" (u8.const 0)) "cannot write after being notified that the readable end dropped")
(component instance $i6.1 $Tester)
(assert_trap (invoke "trap-after-stream-reader-eager-dropped" (u8.const 1)) "cannot write after being notified that the readable end dropped")
(component instance $i7.0 $Tester)
(assert_trap (invoke "trap-after-stream-reader-async-dropped" (u8.const 0)) "cannot write after being notified that the readable end dropped")
(component instance $i7.1 $Tester)
(assert_trap (invoke "trap-after-stream-reader-async-dropped" (u8.const 1)) "cannot write after being notified that the readable end dropped")
(component instance $i8.0 $Tester)

(assert_trap (invoke "trap-after-stream-writer-eager-dropped" (u8.const 0)) "cannot read after being notified that the writable end dropped")
(component instance $i8.1 $Tester)
(assert_trap (invoke "trap-after-stream-writer-eager-dropped" (u8.const 1)) "cannot read after being notified that the writable end dropped")
(component instance $i8.2 $Tester)
(assert_trap (invoke "trap-after-stream-writer-eager-dropped" (u8.const 2)) "cannot lift stream after being notified that the writable end dropped")
(component instance $i9.0 $Tester)
(assert_trap (invoke "trap-after-stream-writer-async-dropped" (u8.const 0)) "cannot read after being notified that the writable end dropped")
(component instance $i9.1 $Tester)
(assert_trap (invoke "trap-after-stream-writer-async-dropped" (u8.const 1)) "cannot read after being notified that the writable end dropped")
(component instance $i9.2 $Tester)
(assert_trap (invoke "trap-after-stream-writer-async-dropped" (u8.const 2)) "cannot lift stream after being notified that the writable end dropped")

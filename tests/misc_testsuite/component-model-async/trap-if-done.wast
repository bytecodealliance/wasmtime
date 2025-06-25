;;! component_model_async = true

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
      (import "" "future.drop-writable" (func $future.drop-writable (param i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
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
      (func $acknowledge-future-write (export "acknowledge-future-write")
        ;; confirm we got a FUTURE_WRITE $writable-end COMPLETED event
        (local $ret i32)
        (local.set $ret (call $waitable-set.wait (global.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 5 (; FUTURE_WRITE ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (global.get $writable-end) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (i32.load (i32.const 4)))
          (then unreachable))
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
    (canon future.drop-writable $FT (core func $future.drop-writable))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.write $ST async (memory $memory "mem") (core func $stream.write))
    (canon stream.drop-writable $ST (core func $stream.drop-writable))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "future.drop-writable" (func $future.drop-writable))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))
      (export "stream.drop-writable" (func $stream.drop-writable))
    ))))
    (func (export "start-future") (result (future u8)) (canon lift (core func $cm "start-future")))
    (func (export "future-write") (result u32) (canon lift (core func $cm "future-write")))
    (func (export "acknowledge-future-write") (canon lift (core func $cm "acknowledge-future-write")))
    (func (export "future-drop-writable") (canon lift (core func $cm "future-drop-writable")))
    (func (export "start-stream") (result (stream u8)) (canon lift (core func $cm "start-stream")))
    (func (export "stream-write") (result u32) (canon lift (core func $cm "stream-write")))
    (func (export "acknowledge-stream-write") (canon lift (core func $cm "acknowledge-stream-write")))
    (func (export "stream-drop-writable") (canon lift (core func $cm "stream-drop-writable")))
  )
  (component $D
    (import "c" (instance $c
      (export "start-future" (func (result (future u8))))
      (export "future-write" (func (result u32)))
      (export "acknowledge-future-write" (func))
      (export "future-drop-writable" (func))
      (export "start-stream" (func (result (stream u8))))
      (export "stream-write" (func (result u32)))
      (export "acknowledge-stream-write" (func))
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
      (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
      (import "" "start-future" (func $start-future (result i32)))
      (import "" "future-write" (func $future-write (result i32)))
      (import "" "acknowledge-future-write" (func $acknowledge-future-write))
      (import "" "future-drop-writable" (func $future-drop-writable))
      (import "" "start-stream" (func $start-stream (result i32)))
      (import "" "stream-write" (func $stream-write (result i32)))
      (import "" "acknowledge-stream-write" (func $acknowledge-stream-write))
      (import "" "stream-drop-writable" (func $stream-drop-writable))

      (func $trap-after-future-eager-write (export "trap-after-future-eager-write")
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; start a read on our end so the next write will succeed
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; calling future.write in $C should succeed eagerly
        (local.set $ret (call $future-write))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        ;; calling future.write in $C now should trap
        (drop (call $future-write))
      )
      (func $trap-after-future-async-write (export "trap-after-future-async-write")
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; calling future.write in $C should block
        (local.set $ret (call $future-write))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; our future.read should then succeed eagerly
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        ;; let $C see the write completed so the future is 'done'
        (call $acknowledge-future-write)

        ;; trying to call future.write again in $C should trap
        (drop (call $future-write))
      )
      (func $trap-after-future-reader-dropped (export "trap-after-future-reader-dropped")
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; drop our readable end before writer can write
        (call $future.drop-readable (local.get $fr))

        ;; let $C try to future.write and find out we DROPPED
        (local.set $ret (call $future-write))
        (if (i32.ne (i32.const 1 (; DROPPED ;)) (local.get $ret))
          (then unreachable))

        ;; trying to call future.write again in $C should trap
        (drop (call $future-write))
      )
      (func $trap-after-future-eager-read (export "trap-after-future-eager-read") (param $bool i32) (result i32)
        (local $ret i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; calling future.write in $C should block
        (local.set $ret (call $future-write))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; our future.read should then succeed eagerly
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        (if (i32.eqz (local.get $bool)) (then
          ;; calling future.read again should then trap
          (drop (call $future.read (local.get $fr) (i32.const 16)))
        ) (else
          ;; lifting the future by returning it should also trap
          (return (local.get $fr))
        ))
        unreachable
      )
      (func $trap-after-future-async-read (export "trap-after-future-async-read") (param $bool i32) (result i32)
        (local $ret i32) (local $ws i32)
        (local $fr i32)
        (local.set $fr (call $start-future))

        ;; read first, so it blocks
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; calling future.write in $C should then succeed eagerly
        (local.set $ret (call $future-write))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

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

        (if (i32.eqz (local.get $bool)) (then
          ;; calling future.read again should then trap
          (drop (call $future.read (local.get $fr) (i32.const 16)))
        ) (else
          ;; lifting the future by returning it should also trap
          (return (local.get $fr))
        ))
        unreachable
      )
      (func $trap-after-stream-reader-eager-dropped (export "trap-after-stream-reader-eager-dropped")
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
        (drop (call $stream-write))
      )
      (func $trap-after-stream-reader-async-dropped (export "trap-after-stream-reader-async-dropped")
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
        (drop (call $stream-write))
      )
      (func $trap-after-stream-writer-eager-dropped (export "trap-after-stream-writer-eager-dropped") (param $bool i32) (result i32)
        (local $ret i32)
        (local $sr i32)
        (local.set $sr (call $start-stream))

        ;; immediately drop the writable end
        (call $stream-drop-writable)

        ;; calling stream.read will see that the writer dropped
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))
        (if (i32.ne (i32.const 0x01 (; DROPPED=1 | (0<<4) ;)) (local.get $ret))
          (then unreachable))

        (if (i32.eqz (local.get $bool)) (then
          ;; calling stream.read again should then trap
          (drop (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))
        ) (else
          ;; lifting the stream by returning it should also trap
          (return (local.get $sr))
        ))
        unreachable
      )
      (func $trap-after-stream-writer-async-dropped (export "trap-after-stream-writer-async-dropped") (param $bool i32) (result i32)
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

        (if (i32.eqz (local.get $bool)) (then
          ;; calling stream.read again should then trap
          (drop (call $stream.read (local.get $sr) (i32.const 16) (i32.const 100)))
        ) (else
          ;; lifting the stream by returning it should also trap
          (return (local.get $sr))
        ))
        unreachable
      )
    )
    (type $FT (future u8))
    (type $ST (stream u8))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon future.new $FT (core func $future.new))
    (canon future.read $FT async (memory $memory "mem") (core func $future.read))
    (canon future.drop-readable $FT (core func $future.drop-readable))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.read $ST async (memory $memory "mem") (core func $stream.read))
    (canon stream.drop-readable $ST (core func $stream.drop-readable))
    (canon lower (func $c "start-future") (core func $start-future'))
    (canon lower (func $c "future-write") (core func $future-write'))
    (canon lower (func $c "acknowledge-future-write") (core func $acknowledge-future-write'))
    (canon lower (func $c "future-drop-writable") (core func $future-drop-writable'))
    (canon lower (func $c "start-stream") (core func $start-stream'))
    (canon lower (func $c "stream-write") (core func $stream-write'))
    (canon lower (func $c "acknowledge-stream-write") (core func $acknowledge-stream-write'))
    (canon lower (func $c "stream-drop-writable") (core func $stream-drop-writable'))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.drop-readable" (func $future.drop-readable))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "start-future" (func $start-future'))
      (export "future-write" (func $future-write'))
      (export "acknowledge-future-write" (func $acknowledge-future-write'))
      (export "future-drop-writable" (func $future-drop-writable'))
      (export "start-stream" (func $start-stream'))
      (export "stream-write" (func $stream-write'))
      (export "acknowledge-stream-write" (func $acknowledge-stream-write'))
      (export "stream-drop-writable" (func $stream-drop-writable'))
    ))))
    (func (export "trap-after-future-eager-write") (canon lift (core func $core "trap-after-future-eager-write")))
    (func (export "trap-after-future-async-write") (canon lift (core func $core "trap-after-future-async-write")))
    (func (export "trap-after-future-reader-dropped") (canon lift (core func $core "trap-after-future-reader-dropped")))
    (func (export "trap-after-future-eager-read") (param "bool" bool) (result $FT) (canon lift (core func $core "trap-after-future-eager-read")))
    (func (export "trap-after-future-async-read") (param "bool" bool) (result $FT) (canon lift (core func $core "trap-after-future-async-read")))
    (func (export "trap-after-stream-reader-eager-dropped") (canon lift (core func $core "trap-after-stream-reader-eager-dropped")))
    (func (export "trap-after-stream-reader-async-dropped") (canon lift (core func $core "trap-after-stream-reader-async-dropped")))
    (func (export "trap-after-stream-writer-eager-dropped") (param "bool" bool) (result $ST) (canon lift (core func $core "trap-after-stream-writer-eager-dropped")))
    (func (export "trap-after-stream-writer-async-dropped") (param "bool" bool) (result $ST) (canon lift (core func $core "trap-after-stream-writer-async-dropped")))
  )
  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "trap-after-future-eager-write") (alias export $d "trap-after-future-eager-write"))
  (func (export "trap-after-future-async-write") (alias export $d "trap-after-future-async-write"))
  (func (export "trap-after-future-reader-dropped") (alias export $d "trap-after-future-reader-dropped"))
  (func (export "trap-after-future-eager-read") (alias export $d "trap-after-future-eager-read"))
  (func (export "trap-after-future-async-read") (alias export $d "trap-after-future-async-read"))
  (func (export "trap-after-stream-reader-eager-dropped") (alias export $d "trap-after-stream-reader-eager-dropped"))
  (func (export "trap-after-stream-reader-async-dropped") (alias export $d "trap-after-stream-reader-async-dropped"))
  (func (export "trap-after-stream-writer-eager-dropped") (alias export $d "trap-after-stream-writer-eager-dropped"))
  (func (export "trap-after-stream-writer-async-dropped") (alias export $d "trap-after-stream-writer-async-dropped"))
)

(component instance $i1 $Tester)
(assert_trap (invoke "trap-after-future-eager-write") "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i2 $Tester)
(assert_trap (invoke "trap-after-future-async-write") "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i3 $Tester)
(assert_trap (invoke "trap-after-future-reader-dropped") "cannot write to future after previous write succeeded or readable end dropped")
(component instance $i4.1 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (bool.const false)) "cannot read from future after previous read succeeded")
(component instance $i4.2 $Tester)
(assert_trap (invoke "trap-after-future-eager-read" (bool.const true)) "cannot lift future after previous read succeeded")
(component instance $i5.1 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (bool.const false)) "cannot read from future after previous read succeeded")
(component instance $i5.2 $Tester)
(assert_trap (invoke "trap-after-future-async-read" (bool.const true)) "cannot lift future after previous read succeeded")
(component instance $i6 $Tester)
(assert_trap (invoke "trap-after-stream-reader-eager-dropped") "cannot write to stream after being notified that the readable end dropped")
(component instance $i7 $Tester)
(assert_trap (invoke "trap-after-stream-reader-async-dropped") "cannot write to stream after being notified that the readable end dropped")
(component instance $i8.1 $Tester)
(assert_trap (invoke "trap-after-stream-writer-eager-dropped" (bool.const false)) "cannot read from stream after being notified that the writable end dropped")
(component instance $i8.2 $Tester)
(assert_trap (invoke "trap-after-stream-writer-eager-dropped" (bool.const true)) "cannot lift stream after being notified that the writable end dropped")
(component instance $i9.1 $Tester)
(assert_trap (invoke "trap-after-stream-writer-async-dropped" (bool.const false)) "cannot read from stream after being notified that the writable end dropped")
(component instance $i9.2 $Tester)
(assert_trap (invoke "trap-after-stream-writer-async-dropped" (bool.const true)) "cannot lift stream after being notified that the writable end dropped")

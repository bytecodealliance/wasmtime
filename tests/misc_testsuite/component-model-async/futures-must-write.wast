;;! component_model_async = true

;; This test contains two components $C and $D that test that a trap occurs
;; when closing the writable end of a future (in $C) before having written
;; a value while closing the readable end of a future (in $D) before reading
;; a value is fine.
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/future-trap/test/async/futures-must-write.wast)
(component
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "future.drop-writable" (func $future.drop-writable (param i32)))

      (global $fw (mut i32) (i32.const 0))

      (func $start-future (export "start-future") (result i32)
        ;; create a new future, return the readable end to the caller
        (local $ret64 i64)
        (local.set $ret64 (call $future.new))
        (global.set $fw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (i32.wrap_i64 (local.get $ret64))
      )
      (func $attempt-write (export "attempt-write") (result i32)
        ;; because the caller already dropped the readable end, this write will eagerly
        ;; return DROPPED having written no values.
        (local $ret i32)
        (local.set $ret (call $future.write (global.get $fw) (i32.const 42)))
        (if (i32.ne (i32.const 0x01 (; DROPPED=1 | (0<<4) ;)) (local.get $ret))
          (then
            (i32.load (i32.add (local.get $ret) (i32.const 0x8000_0000)))
          unreachable))

        ;; return without trapping
        (i32.const 42)
      )
      (func $drop-writable (export "drop-writable")
        ;; maybe boom
        (call $future.drop-writable (global.get $fw))
      )
    )
    (type $FT (future u8))
    (canon future.new $FT (core func $future.new))
    (canon future.write $FT async (memory $memory "mem") (core func $future.write))
    (canon future.drop-writable $FT (core func $future.drop-writable))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "future.drop-writable" (func $future.drop-writable))
    ))))
    (func (export "start-future") (result (future u8)) (canon lift (core func $cm "start-future")))
    (func (export "attempt-write") (result u32) (canon lift (core func $cm "attempt-write")))
    (func (export "drop-writable") (canon lift (core func $cm "drop-writable")))
  )
  (component $D
    (import "c" (instance $c
      (export "start-future" (func (result (future u8))))
      (export "attempt-write" (func (result u32)))
      (export "drop-writable" (func))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $Core
      (import "" "mem" (memory 1))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
      (import "" "start-future" (func $start-future (result i32)))
      (import "" "attempt-write" (func $attempt-write (result i32)))
      (import "" "drop-writable" (func $drop-writable))

      (func $drop-readable-future-before-read (export "drop-readable-future-before-read") (result i32)
        ;; call 'start-future' to get the future we'll be working with
        (local $fr i32)
        (local.set $fr (call $start-future))
        (if (i32.ne (i32.const 1) (local.get $fr))
          (then unreachable))

        ;; ok to immediately drop the readable end
        (call $future.drop-readable (local.get $fr))

        ;; the callee will see that we dropped the readable end when it tries to write
        (call $attempt-write)
      )
      (func $drop-writable-future-before-write (export "drop-writable-future-before-write")
        ;; call 'start-future' to get the future we'll be working with
        (local $fr i32)
        (local.set $fr (call $start-future))
        (if (i32.ne (i32.const 1) (local.get $fr))
          (then unreachable))

        ;; boom
        (call $drop-writable)
      )
    )
    (type $FT (future u8))
    (canon future.new $FT (core func $future.new))
    (canon future.read $FT async (memory $memory "mem") (core func $future.read))
    (canon future.drop-readable $FT (core func $future.drop-readable))
    (canon lower (func $c "start-future") (core func $start-future'))
    (canon lower (func $c "attempt-write") (core func $attempt-write'))
    (canon lower (func $c "drop-writable") (core func $drop-writable'))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.drop-readable" (func $future.drop-readable))
      (export "start-future" (func $start-future'))
      (export "attempt-write" (func $attempt-write'))
      (export "drop-writable" (func $drop-writable'))
    ))))
    (func (export "drop-readable-future-before-read") (result u32) (canon lift (core func $core "drop-readable-future-before-read")))
    (func (export "drop-writable-future-before-write") (canon lift (core func $core "drop-writable-future-before-write")))
  )
  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "drop-writable-future-before-write") (alias export $d "drop-writable-future-before-write"))
  (func (export "drop-readable-future-before-read") (alias export $d "drop-readable-future-before-read"))
)

(assert_return (invoke "drop-readable-future-before-read") (u32.const 42))
(assert_trap (invoke "drop-writable-future-before-write") "cannot drop future write end without first writing a value")

;; Drives every export of the `dep` preload, which lives under an interface
;; name, covering what crosses the host trampoline that forwards each export.
;;
;; Borrows are passed bare, nested in a `tuple` (flat) and nested in a `list`
;; (through linear memory); a borrow lifted but not dropped would trip
;; `validate_scope_exit` when the call returns. One handle is dropped while
;; another is live, and the preload's destructor runs on a task of its own, so
;; the `context` slot set here must survive it. `next` is called twice and
;; combined as `first * 10 + second`, which yields 12 only if both calls reach
;; the same preload instance; two instantiations would each return 1. The last
;; three calls hand over a `future`, a `stream` and an `error-context`.
;;
;; Each borrow's value is folded into the result, so a borrow resolving to the
;; wrong resource changes the answer. Expects 100120.
(component
  (import "dep" (instance $dep
    (export "a:b/c@0.1.0" (instance
      (export "thing" (type (sub resource)))
      (export "make" (func (result (own 0))))
      (export "value" (func (param "r" (borrow 0)) (result u32)))
      (export "pair" (func (param "p" (tuple (borrow 0) u32)) (result u32)))
      (export "sum" (func (param "p" (list (borrow 0))) (result u32)))
      (export "dropped" (func (result u32)))
      (export "next" (func (result u32)))
      (export "take-future" (func (param "f" (future u32)) (result u32)))
      (export "take-stream" (func (param "s" (stream u8)) (result u32)))
      (export "take-errctx" (func (param "e" error-context) (result u32)))))))
  (alias export $dep "a:b/c@0.1.0" (instance $c))
  (alias export $c "thing" (type $t))

  (core module $libc
    (memory (export "mem") 1)
    (data (i32.const 0) "hello")
    (data (i32.const 16) "\01\02\03\04")
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) (i32.const 64)))
  (core instance $libc (instantiate $libc))

  (type $f (future u32))
  (type $s (stream u8))
  (core func $fnew (canon future.new $f))
  (core func $fwrite (canon future.write $f async (memory (core memory $libc "mem"))))
  (core func $snew (canon stream.new $s))
  (core func $swrite (canon stream.write $s async (memory (core memory $libc "mem"))))
  (core func $ecnew (canon error-context.new (memory (core memory $libc "mem"))))

  (core func $make (canon lower (func $c "make")))
  (core func $value (canon lower (func $c "value")))
  (core func $pair (canon lower (func $c "pair")))
  (core func $sum (canon lower (func $c "sum")
    (memory (core memory $libc "mem"))
    (realloc (core func $libc "realloc"))))
  (core func $dropped (canon lower (func $c "dropped")))
  (core func $next (canon lower (func $c "next")))
  (core func $take_future (canon lower (func $c "take-future")))
  (core func $take_stream (canon lower (func $c "take-stream")))
  (core func $take_errctx (canon lower (func $c "take-errctx")))
  (core func $drop (canon resource.drop $t))
  (core func $cget (canon context.get i32 0))
  (core func $cset (canon context.set i32 0))

  (core module $m
    (import "" "make" (func $make (result i32)))
    (import "" "value" (func $value (param i32) (result i32)))
    (import "" "pair" (func $pair (param i32 i32) (result i32)))
    (import "" "sum" (func $sum (param i32 i32) (result i32)))
    (import "" "dropped" (func $dropped (result i32)))
    (import "" "next" (func $next (result i32)))
    (import "" "take-future" (func $take_future (param i32) (result i32)))
    (import "" "take-stream" (func $take_stream (param i32) (result i32)))
    (import "" "take-errctx" (func $take_errctx (param i32) (result i32)))
    (import "" "drop" (func $drop (param i32)))
    (import "" "fnew" (func $fnew (result i64)))
    (import "" "fwrite" (func $fwrite (param i32 i32) (result i32)))
    (import "" "snew" (func $snew (result i64)))
    (import "" "swrite" (func $swrite (param i32 i32 i32) (result i32)))
    (import "" "ecnew" (func $ecnew (param i32 i32) (result i32)))
    (import "" "cget" (func $cget (result i32)))
    (import "" "cset" (func $cset (param i32)))
    (import "" "mem" (memory 1))

    (func (export "run") (result i32)
      (local $a i32) (local $b i32) (local $va i32) (local $vb i32)
      (local $pair i64) (local $r i32) (local $w i32)

      (call $cset (i32.const 0x1234))

      (local.set $a (call $make))
      (local.set $b (call $make))

      ;; Borrow both while both handles are live.
      (local.set $va (call $value (local.get $a)))
      (local.set $vb (call $value (local.get $b)))
      (if (i32.ne (local.get $va) (i32.const 10)) (then unreachable))
      (if (i32.ne (local.get $vb) (i32.const 20)) (then unreachable))

      ;; A borrow nested in a tuple: 1 * 10 + 3.
      (if (i32.ne (call $pair (local.get $a) (i32.const 3)) (i32.const 13))
        (then unreachable))

      ;; Borrows nested in a list: 100 * 2 + (1 + 2).
      (i32.store (i32.const 32) (local.get $a))
      (i32.store (i32.const 36) (local.get $b))
      (if (i32.ne (call $sum (i32.const 32) (i32.const 2)) (i32.const 203))
        (then unreachable))

      ;; Nothing dropped yet: the borrows above must not have consumed either
      ;; handle.
      (if (i32.ne (call $dropped) (i32.const 0)) (then unreachable))

      ;; Drop one handle; the other must stay usable and keep its identity.
      (call $drop (local.get $a))
      (if (i32.ne (call $dropped) (i32.const 1)) (then unreachable))
      (if (i32.ne (call $value (local.get $b)) (i32.const 20)) (then unreachable))

      (call $drop (local.get $b))
      (if (i32.ne (call $dropped) (i32.const 2)) (then unreachable))

      ;; Our slot must survive the destructors untouched.
      (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))

      ;; The write completes eagerly, so the preload's sync `future.read`
      ;; succeeds immediately.
      (local.set $pair (call $fnew))
      (local.set $r (i32.wrap_i64 (local.get $pair)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
      (i32.store (i32.const 48) (i32.const 42))
      (drop (call $fwrite (local.get $w) (i32.const 48)))
      (if (i32.ne (call $take_future (local.get $r)) (i32.const 42))
        (then unreachable))

      ;; Writes 1,2,3,4; the preload returns their sum.
      (local.set $pair (call $snew))
      (local.set $r (i32.wrap_i64 (local.get $pair)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
      (drop (call $swrite (local.get $w) (i32.const 16) (i32.const 4)))
      (if (i32.ne (call $take_stream (local.get $r)) (i32.const 10))
        (then unreachable))

      ;; A 5-byte debug message.
      (if (i32.ne (call $take_errctx (call $ecnew (i32.const 0) (i32.const 5)))
                  (i32.const 5))
        (then unreachable))

      ;; Two calls into one shared instance: 1 * 10 + 2.
      (if (i32.ne
            (i32.add (i32.mul (call $next) (i32.const 10)) (call $next))
            (i32.const 12))
        (then unreachable))

      (i32.add (i32.const 100000)
        (i32.add (i32.mul (local.get $va) (i32.const 10)) (local.get $vb))))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "make" (func $make))
    (export "value" (func $value))
    (export "pair" (func $pair))
    (export "sum" (func $sum))
    (export "dropped" (func $dropped))
    (export "next" (func $next))
    (export "take-future" (func $take_future))
    (export "take-stream" (func $take_stream))
    (export "take-errctx" (func $take_errctx))
    (export "drop" (func $drop))
    (export "fnew" (func $fnew))
    (export "fwrite" (func $fwrite))
    (export "snew" (func $snew))
    (export "swrite" (func $swrite))
    (export "ecnew" (func $ecnew))
    (export "cget" (func $cget))
    (export "cset" (func $cset))
    (export "mem" (memory $libc "mem"))))))

  (func (export "run") (result u32) (canon lift (core func $i "run")))
)

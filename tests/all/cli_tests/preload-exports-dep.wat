;; A `--preload` exercised by `preload-exports-main.wat`, covering the kinds of
;; export and value that cross the host trampoline forwarding each export.
;;
;; Everything lives under an interface name, checking that `a:b/c@0.1.0` is
;; forwarded as a single opaque key rather than split on `:` or `/`.
;;
;; `thing` is a resource: `make` hands out `own`ed handles, and `value`, `pair`
;; and `sum` each fold a borrow's representation into their result, so a borrow
;; resolving to the wrong resource changes the answer. `pair` takes its borrow
;; inside a `tuple`, which is passed flat, and `sum` inside a `list`, which
;; travels through linear memory. `next` holds mutable state, so repeated calls
;; show the preload is instantiated once and shared. The rest take a `future`,
;; a `stream` and an `error-context`.
(component
  (core module $libc
    (memory (export "mem") 1)
    ;; Bump allocator: the debug message is written past our scratch area.
    (global $next (mut i32) (i32.const 64))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $p i32)
      (local.set $p (global.get $next))
      (global.set $next (i32.add (global.get $next) (local.get 3)))
      (local.get $p)))
  (core instance $libc (instantiate $libc))

  (type $f (future u32))
  (type $s (stream u8))
  (core func $fread (canon future.read $f async (memory (core memory $libc "mem"))))
  (core func $sread (canon stream.read $s async (memory (core memory $libc "mem"))))
  (core func $ecmsg (canon error-context.debug-message
    (memory (core memory $libc "mem"))
    (realloc (core func $libc "realloc"))))
  (core func $cget (canon context.get i32 0))
  (core func $cset (canon context.set i32 0))

  (core module $m
    (import "" "fread" (func $fread (param i32 i32) (result i32)))
    (import "" "sread" (func $sread (param i32 i32 i32) (result i32)))
    (import "" "ecmsg" (func $ecmsg (param i32 i32)))
    (import "" "cget" (func $cget (result i32)))
    (import "" "cset" (func $cset (param i32)))
    (import "" "mem" (memory 1))

    (global $next (mut i32) (i32.const 0))
    (global $counter (mut i32) (i32.const 0))
    (global $dropped (mut i32) (i32.const 0))

    (func (export "make") (result i32)
      (global.set $next (i32.add (global.get $next) (i32.const 1)))
      (global.get $next))

    ;; A borrow lowered into the defining instance arrives as the raw
    ;; representation, not a handle.
    (func (export "value") (param i32) (result i32)
      (i32.mul (local.get 0) (i32.const 10)))

    (func (export "pair") (param i32 i32) (result i32)
      (i32.add (i32.mul (local.get 0) (i32.const 10)) (local.get 1)))

    ;; Takes the (ptr, len) of a list of borrows. Returns 100 * len plus the
    ;; sum of their representations.
    (func (export "sum") (param $ptr i32) (param $len i32) (result i32)
      (local $i i32) (local $acc i32)
      (loop $loop
        (if (i32.lt_u (local.get $i) (local.get $len))
          (then
            (local.set $acc
              (i32.add (local.get $acc)
                (i32.load (i32.add (local.get $ptr)
                                   (i32.mul (local.get $i) (i32.const 4))))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $loop))))
      (i32.add (i32.mul (local.get $len) (i32.const 100)) (local.get $acc)))

    (func (export "dropped") (result i32) (global.get $dropped))

    ;; Runs on a task of its own, so the slot starts zeroed whatever the caller
    ;; left in theirs. The sentinel written here would be visible to the caller
    ;; if the two shared a task.
    (func (export "dtor") (param i32)
      (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
      (call $cset (i32.const 0x5678))
      (global.set $dropped (i32.add (global.get $dropped) (i32.const 1))))

    (func (export "next") (result i32)
      (global.set $counter (i32.add (global.get $counter) (i32.const 1)))
      (global.get $counter))

    ;; `future.read` completes immediately because the main component writes
    ;; the value before handing the future over.
    (func (export "take-future") (param $fut i32) (result i32)
      (local $r i32)
      (local.set $r (call $fread (local.get $fut) (i32.const 0)))
      (if (i32.eq (local.get $r) (i32.const -1)) (then unreachable))
      (i32.load (i32.const 0)))

    ;; Reads 4 bytes and returns their sum.
    (func (export "take-stream") (param $st i32) (result i32)
      (local $r i32)
      (local.set $r (call $sread (local.get $st) (i32.const 0) (i32.const 4)))
      (if (i32.eq (local.get $r) (i32.const -1)) (then unreachable))
      (i32.add
        (i32.add (i32.load8_u (i32.const 0)) (i32.load8_u (i32.const 1)))
        (i32.add (i32.load8_u (i32.const 2)) (i32.load8_u (i32.const 3)))))

    ;; Returns the length of the debug message, confirming the re-indexing into
    ;; this component's own table.
    (func (export "take-errctx") (param $ec i32) (result i32)
      ;; Write the (ptr,len) pair at offset 0, then return the len.
      (call $ecmsg (local.get $ec) (i32.const 0))
      (i32.load (i32.const 4)))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "fread" (func $fread))
    (export "sread" (func $sread))
    (export "ecmsg" (func $ecmsg))
    (export "cget" (func $cget))
    (export "cset" (func $cset))
    (export "mem" (memory $libc "mem"))))))

  (type $r (resource (rep i32) (dtor (core func $i "dtor"))))
  (core func $new (canon resource.new $r))

  (core module $w
    (import "" "make" (func $make (result i32)))
    (import "" "new" (func $new (param i32) (result i32)))
    (func (export "make") (result i32) (call $new (call $make)))
  )
  (core instance $wi (instantiate $w (with "" (instance
    (export "make" (func $i "make"))
    (export "new" (func $new))))))

  (func $make (result (own $r)) (canon lift (core func $wi "make")))
  (func $value (param "r" (borrow $r)) (result u32) (canon lift (core func $i "value")))
  (func $pair (param "p" (tuple (borrow $r) u32)) (result u32)
    (canon lift (core func $i "pair")))
  (func $sum (param "p" (list (borrow $r))) (result u32)
    (canon lift (core func $i "sum")
      (memory (core memory $libc "mem"))
      (realloc (core func $libc "realloc"))))
  (func $dropped (result u32) (canon lift (core func $i "dropped")))
  (func $next (result u32) (canon lift (core func $i "next")))
  (func $take_future (param "f" (future u32)) (result u32)
    (canon lift (core func $i "take-future")))
  (func $take_stream (param "s" (stream u8)) (result u32)
    (canon lift (core func $i "take-stream")))
  (func $take_errctx (param "e" error-context) (result u32)
    (canon lift (core func $i "take-errctx")))

  (instance $inst
    (export "thing" (type $r))
    (export "make" (func $make))
    (export "value" (func $value))
    (export "pair" (func $pair))
    (export "sum" (func $sum))
    (export "dropped" (func $dropped))
    (export "next" (func $next))
    (export "take-future" (func $take_future))
    (export "take-stream" (func $take_stream))
    (export "take-errctx" (func $take_errctx)))
  (export "a:b/c@0.1.0" (instance $inst))
)

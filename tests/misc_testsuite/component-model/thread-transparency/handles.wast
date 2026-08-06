;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! component_model_error_context = true

;; A handle anywhere in either of an adapter's signatures makes it opaque.

;; An `own` handle as a parameter, and again nested inside an aggregate.
(component
  (component $A
    (type $t (resource (rep i32)))
    (export $t' "t" (type $t))
    (core func $rep (canon resource.rep $t))
    (core func $drop (canon resource.drop $t))
    (core func $new (canon resource.new $t))
    (core module $M
      (import "" "rep" (func $rep (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      ;; Takes ownership, reads through it, then drops it.
      (func $take (export "take") (param i32) (result i32)
        (local $r i32)
        (local.set $r (call $rep (local.get 0)))
        (call $drop (local.get 0))
        (local.get $r))
      ;; Same, but the handle arrived as the second field of a tuple.
      (func (export "take-nested") (param i32 i32) (result i32)
        (i32.add (local.get 0) (call $take (local.get 1))))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "rep" (func $rep))
      (export "drop" (func $drop))))))
    (core module $Ctor
      (import "" "new" (func $new (param i32) (result i32)))
      (func (export "make") (param i32) (result i32)
        (call $new (local.get 0)))
    )
    (core instance $ctor (instantiate $Ctor
      (with "" (instance (export "new" (func $new))))))
    (func (export "make") (param "rep" u32) (result (own $t'))
      (canon lift (core func $ctor "make")))
    (func (export "take") (param "h" (own $t')) (result u32)
      (canon lift (core func $m "take")))
    (func (export "take-nested") (param "x" (tuple u32 (own $t'))) (result u32)
      (canon lift (core func $m "take-nested")))
  )

  (component $B
    (import "t" (type $t (sub resource)))
    (import "make" (func $make (param "rep" u32) (result (own $t))))
    (import "take" (func $take (param "h" (own $t)) (result u32)))
    (import "take-nested"
      (func $take-nested (param "x" (tuple u32 (own $t))) (result u32)))
    (core func $make' (canon lower (func $make)))
    (core func $take' (canon lower (func $take)))
    (core func $take-nested' (canon lower (func $take-nested)))
    (core module $N
      (import "" "make" (func $make (param i32) (result i32)))
      (import "" "take" (func $take (param i32) (result i32)))
      (import "" "take-nested" (func $take-nested (param i32 i32) (result i32)))
      (func (export "g") (result i32)
        ;; An `own` result on the way out, an `own` param on the way back in,
        ;; and once more nested inside a tuple.
        (i32.add
          (call $take (call $make (i32.const 100)))
          (call $take-nested (i32.const 5) (call $make (i32.const 200)))))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "make" (func $make'))
      (export "take" (func $take'))
      (export "take-nested" (func $take-nested'))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B
    (with "t" (type $a "t"))
    (with "make" (func $a "make"))
    (with "take" (func $a "take"))
    (with "take-nested" (func $a "take-nested"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 305))

;; A `borrow` handle as a parameter: the caller keeps ownership across the call.
(component
  (component $A
    (type $t (resource (rep i32)))
    (export $t' "t" (type $t))
    (core func $new (canon resource.new $t))
    (core func $drop (canon resource.drop $t))
    (core module $M
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (func (export "make") (param i32) (result i32)
        (call $new (local.get 0)))
      ;; A `borrow` lifted into the resource's own defining component arrives
      ;; as the representation itself rather than as a table index.
      (func (export "peek") (param i32) (result i32)
        (local.get 0))
      (func (export "release") (param i32)
        (call $drop (local.get 0)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "new" (func $new))
      (export "drop" (func $drop))))))
    (func (export "make") (param "rep" u32) (result (own $t'))
      (canon lift (core func $m "make")))
    (func (export "peek") (param "h" (borrow $t')) (result u32)
      (canon lift (core func $m "peek")))
    (func (export "release") (param "h" (own $t'))
      (canon lift (core func $m "release")))
  )

  (component $B
    (import "t" (type $t (sub resource)))
    (import "make" (func $make (param "rep" u32) (result (own $t))))
    (import "peek" (func $peek (param "h" (borrow $t)) (result u32)))
    (import "release" (func $release (param "h" (own $t))))
    (core func $make' (canon lower (func $make)))
    (core func $peek' (canon lower (func $peek)))
    (core func $release' (canon lower (func $release)))
    (core module $N
      (import "" "make" (func $make (param i32) (result i32)))
      (import "" "peek" (func $peek (param i32) (result i32)))
      (import "" "release" (func $release (param i32)))
      (func (export "g") (result i32)
        (local $h i32) (local $sum i32)
        (local.set $h (call $make (i32.const 7)))
        ;; Borrowing twice is fine; we still own it afterwards.
        (local.set $sum (call $peek (local.get $h)))
        (local.set $sum
          (i32.add (local.get $sum) (call $peek (local.get $h))))
        (call $release (local.get $h))
        (local.get $sum))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "make" (func $make'))
      (export "peek" (func $peek'))
      (export "release" (func $release'))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B
    (with "t" (type $a "t"))
    (with "make" (func $a "make"))
    (with "peek" (func $a "peek"))
    (with "release" (func $a "release"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 14))

;; `future`, `stream`, and `error-context` are handles for this purpose too.
;; The caller hands the readable end over and the callee just receives and drops
;; it, which is enough to run the transfer.
(component
  (type $fut (future u8))
  (type $str (stream u8))

  (component $A
    (type $fut (future u8))
    (type $str (stream u8))
    (core func $future-drop (canon future.drop-readable $fut))
    (core func $stream-drop (canon stream.drop-readable $str))
    (core func $ctx-drop (canon error-context.drop))
    (core module $M
      (import "" "future.drop-readable" (func $future-drop (param i32)))
      (import "" "stream.drop-readable" (func $stream-drop (param i32)))
      (import "" "error-context.drop" (func $ctx-drop (param i32)))
      (func (export "take-future") (param i32) (result i32)
        (call $future-drop (local.get 0))
        (i32.const 1))
      (func (export "take-stream") (param i32) (result i32)
        (call $stream-drop (local.get 0))
        (i32.const 2))
      (func (export "take-error-context") (param i32) (result i32)
        (call $ctx-drop (local.get 0))
        (i32.const 4))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "future.drop-readable" (func $future-drop))
      (export "stream.drop-readable" (func $stream-drop))
      (export "error-context.drop" (func $ctx-drop))))))
    (func (export "take-future") (param "x" (future u8)) (result u32)
      (canon lift (core func $m "take-future")))
    (func (export "take-stream") (param "x" (stream u8)) (result u32)
      (canon lift (core func $m "take-stream")))
    (func (export "take-error-context") (param "x" error-context) (result u32)
      (canon lift (core func $m "take-error-context")))
  )

  (component $B
    (type $fut (future u8))
    (type $str (stream u8))
    (import "take-future" (func $take-future (param "x" (future u8)) (result u32)))
    (import "take-stream" (func $take-stream (param "x" (stream u8)) (result u32)))
    (import "take-error-context"
      (func $take-error-context (param "x" error-context) (result u32)))
    (core module $Libc
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    )
    (core instance $libc (instantiate $Libc))
    (core func $take-future' (canon lower (func $take-future)))
    (core func $take-stream' (canon lower (func $take-stream)))
    (core func $take-error-context' (canon lower (func $take-error-context)))
    (core func $future-new (canon future.new $fut))
    (core func $stream-new (canon stream.new $str))
    (core func $ctx-new
      (canon error-context.new (memory $libc "memory")))
    (core module $N
      (import "" "take-future" (func $take-future (param i32) (result i32)))
      (import "" "take-stream" (func $take-stream (param i32) (result i32)))
      (import "" "take-error-context"
        (func $take-error-context (param i32) (result i32)))
      (import "" "future.new" (func $future-new (result i64)))
      (import "" "stream.new" (func $stream-new (result i64)))
      (import "" "error-context.new" (func $ctx-new (param i32 i32) (result i32)))
      ;; The writable ends stay live: dropping one without writing traps, and
      ;; all this test needs is for the readable ends to make it across.
      (func (export "g") (result i32)
        (local $pair i64) (local $sum i32)
        (local.set $pair (call $future-new))
        (local.set $sum
          (call $take-future (i32.wrap_i64 (local.get $pair))))

        (local.set $pair (call $stream-new))
        (local.set $sum (i32.add (local.get $sum)
          (call $take-stream (i32.wrap_i64 (local.get $pair)))))

        (i32.add (local.get $sum)
          (call $take-error-context
            (call $ctx-new (i32.const 0) (i32.const 0)))))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "take-future" (func $take-future'))
      (export "take-stream" (func $take-stream'))
      (export "take-error-context" (func $take-error-context'))
      (export "future.new" (func $future-new))
      (export "stream.new" (func $stream-new))
      (export "error-context.new" (func $ctx-new))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B
    (with "take-future" (func $a "take-future"))
    (with "take-stream" (func $a "take-stream"))
    (with "take-error-context" (func $a "take-error-context"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 7))

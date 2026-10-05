;;! component_model_async = true
;;! exceptions = true
;;! reference_types = true
;;! multi_memory = true

;; Tests for the interaction of Wasm exceptions and async lifts/lowers.
;;
;; In all cases no matter what interleavings/etc show up, an uncaught exception
;; from one component cannot ever be caught in another component and must show
;; up as a form of uncaught exception trap.

;; sync->async
(component definition $A
  (component $A
    (core module $a
      (tag $t)
      (func (export "run") (param i32) (result i32)
        local.get 0
        if throw $t end
        i32.const 1 (; CALLBACK_CODE_YIELD ;)
      )
      (func (export "cb") (param i32 i32 i32) (result i32) throw $t)
    )
    (core instance $a (instantiate $a))
    (func (export "run") async (param "b" bool)
      (canon lift (core func $a "run") async (callback (core func $a "cb"))))
  )
  (component $B
    (import "a" (instance $a
      (export "run" (func async (param "b" bool)))
    ))

    (core func $run (canon lower (func $a "run")))
    (core module $b
      (import "" "run" (func $run (param i32)))

      (func (export "run") (param i32)
        block $a
          try_table (catch_all $a)
            (call $run (local.get 0))
            return
          end
        end
        unreachable
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(component instance $A $A)
(assert_trap (invoke "run" (bool.const false)) "uncaught exception propagated out of component")
(component instance $A $A)
(assert_trap (invoke "run" (bool.const true)) "uncaught exception propagated out of component")

;; async->sync
(component definition $A
  (component $A
    (core module $a
      (import "" "yield" (func $yield (result i32)))
      (tag $t)
      (func (export "run") (param i32)
        local.get 0
        if call $yield drop end
        throw $t
      )
    )
    (core func $yield (canon thread.yield))
    (core instance $a (instantiate $a
      (with "" (instance
        (export "yield" (func $yield))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $a "run")))
  )
  (component $B
    (import "a" (instance $a
      (export "run" (func async (param "b" bool)))
    ))

    (core func $run (canon lower (func $a "run") async))

    (core module $libc (memory (export "m") 1))
    (core instance $libc (instantiate $libc))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "m"))))

    (core module $b
      (import "" "run" (func $run (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

      (func (export "run") (param i32)
        (local $subtask i32)
        (local $ws i32)
        block $a
          try_table (catch_all $a)
            (local.set $subtask
              (i32.shr_u (call $run (local.get 0)) (i32.const 4)))
            (local.set $ws (call $waitable-set.new))
            (call $waitable.join (local.get $subtask) (local.get $ws))
            (call $waitable-set.wait (local.get $ws) (i32.const 0))
            unreachable
          end
          unreachable
        end
        unreachable
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "waitable-set.wait" (func $waitable-set.wait))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(component instance $A $A)
(assert_trap (invoke "run" (bool.const false)) "uncaught exception propagated out of component")
(component instance $A $A)
(assert_trap (invoke "run" (bool.const true)) "uncaught exception propagated out of component")

;; async->async
(component definition $A
  (component $A
    (core module $a
      (tag $t)
      (func (export "run") (param i32) (result i32)
        local.get 0
        if throw $t end
        i32.const 1 (; CALLBACK_CODE_YIELD ;)
      )
      (func (export "cb") (param i32 i32 i32) (result i32) throw $t)
    )
    (core instance $a (instantiate $a))
    (func (export "run") async (param "b" bool)
      (canon lift (core func $a "run") async (callback (core func $a "cb"))))
  )
  (component $B
    (import "a" (instance $a
      (export "run" (func async (param "b" bool)))
    ))

    (core func $run (canon lower (func $a "run") async))

    (core module $libc (memory (export "m") 1))
    (core instance $libc (instantiate $libc))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "m"))))

    (core module $b
      (import "" "run" (func $run (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

      (func (export "run") (param i32)
        (local $subtask i32)
        (local $ws i32)
        block $a
          try_table (catch_all $a)
            (local.set $subtask
              (i32.shr_u (call $run (local.get 0)) (i32.const 4)))
            (local.set $ws (call $waitable-set.new))
            (call $waitable.join (local.get $subtask) (local.get $ws))
            (call $waitable-set.wait (local.get $ws) (i32.const 0))
            unreachable
          end
          unreachable
        end
        unreachable
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "waitable-set.wait" (func $waitable-set.wait))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(component instance $A $A)
(assert_trap (invoke "run" (bool.const false)) "uncaught exception propagated out of component")
(component instance $A $A)
(assert_trap (invoke "run" (bool.const true)) "uncaught exception propagated out of component")

;; The caller's `realloc` is invoked during the callee's `task.return` to lower
;; a `string` result. An exception thrown by it must trap rather than unwind
;; into the callee's `try_table`.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core module $m
      (import "" "task.return" (func $task.return (param i32 i32)))
      (import "" "mem" (memory 1))
      (data (i32.const 100) "hi")
      (func (export "g") (result i32)
        (block $caught
          (try_table (catch_all $caught)
            (call $task.return (i32.const 100) (i32.const 2)))
          (return (i32.const 0 (; EXIT ;))))
        ;; the caller's exception leaked into this component
        unreachable)
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core func $task.return (canon task.return (result string)
      (memory (core memory $libc "mem"))))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "task.return" (func $task.return))
        (export "mem" (memory $libc "mem"))))))
    (func (export "g") async (result string)
      (canon lift (core func $i "g") async
        (memory (core memory $libc "mem"))
        (callback (core func $i "cb")))))
  (instance $c (instantiate $C))

  (component $A
    (import "g" (func $g async (result string)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core module $r
      (tag $t)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (throw $t)))
    (core instance $r (instantiate $r))
    (core func $g (canon lower (func $g) async
      (memory (core memory $libc "mem"))
      (realloc (core func $r "realloc"))))
    (core module $m
      (import "" "g" (func $g (param i32) (result i32)))
      (func (export "run") (result i32)
        (call $g (i32.const 16))))
    (core instance $i
      (instantiate $m (with "" (instance (export "g" (func $g))))))
    (func (export "run") (result u32) (canon lift (core func $i "run"))))
  (instance $a (instantiate $A (with "g" (func $c "g"))))
  (export "run" (func $a "run")))

(assert_trap (invoke "run") "uncaught exception propagated out of component")

;; The callee's `realloc` is invoked when the caller's call lowers a `string`
;; parameter into the callee. An exception thrown by it must trap rather than
;; unwind into the caller's `try_table`.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core module $r
      (tag $t)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (throw $t)))
    (core instance $r (instantiate $r))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "task.return" (func $task.return))
      (func (export "g") (param i32 i32) (result i32)
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "g") async (param "s" string)
      (canon lift (core func $i "g") async
        (memory (core memory $libc "mem"))
        (realloc (core func $r "realloc"))
        (callback (core func $i "cb")))))
  (instance $c (instantiate $C))

  (component $A
    (import "g" (func $g async (param "s" string)))
    (core module $libc
      (memory (export "mem") 1)
      (data (i32.const 0x100) "hello"))
    (core instance $libc (instantiate $libc))
    (core func $g (canon lower (func $g) async
      (memory (core memory $libc "mem"))))
    (core module $m
      (import "" "g" (func $g (param i32 i32) (result i32)))
      (func (export "run") (result i32)
        (block $caught
          (try_table (catch_all $caught)
            (drop (call $g (i32.const 0x100) (i32.const 5))))
          (return (i32.const 42 (; no leak ;))))
        ;; the callee's exception leaked into this component
        unreachable))
    (core instance $i (instantiate $m
      (with "" (instance (export "g" (func $g))))))
    (func (export "run") (result u32) (canon lift (core func $i "run"))))
  (instance $a (instantiate $A (with "g" (func $c "g"))))
  (export "run" (func $a "run")))

(assert_trap (invoke "run") "uncaught exception propagated out of component")

;; A sync-lift `post-return` function that throws. The host runs it after the
;; export returns; an exception escaping it is not caught by anything on the
;; stack, so it must become an "uncaught exception" trap. (This path is sync,
;; but the same `call_post_return` is used when a sync-lifted export is driven
;; by the async event loop.)
(component
  (core module $r
    (tag $t)
    (func (export "post-return") (param i32)
      (throw $t)))
  (core instance $r (instantiate $r))
  (core module $m
    (func (export "g") (result i32) (i32.const 0)))
  (core instance $i (instantiate $m))
  (func (export "g") (result u32)
    (canon lift (core func $i "g")
      (post-return (core func $r "post-return")))))

(assert_trap (invoke "g") "uncaught exception propagated out of component")

;; The callee's `realloc`, invoked by the host to lower a `string` argument
;; into the callee, throws. This lowering runs in host Rust code
;; (`LowerContext::realloc`), not a fused adapter, so the exception must be
;; converted to a trap there.
(component
  (core module $r
    (tag $t)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (throw $t)))
  (core instance $r (instantiate $r))
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (func (export "g") (param i32 i32) (result i32) (i32.const 0)))
  (core instance $i (instantiate $m))
  (func (export "g") (param "s" string) (result u32)
    (canon lift (core func $i "g")
      (memory (core memory $libc "mem"))
      (realloc (core func $r "realloc")))))

(assert_trap (invoke "g" (str.const "hello"))
  "uncaught exception propagated out of component")

;; The reader's `realloc` is invoked by the host while a `stream.write` copies a
;; `string` into the reader. An exception thrown by it must trap rather than
;; unwind into the writer's `try_table`. Unlike the adapter cases above this
;; `realloc` runs in host Rust code (the stream copy), not inside an adapter.
;; The reader stays alive (yields) so its read is still pending when the write
;; drives the copy.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (type $ST (stream string))
    (core module $r
      (tag $t)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (throw $t)))
    (core instance $r (instantiate $r))
    (core func $stream.read
      (canon stream.read $ST async (memory (core memory $libc "mem"))
        (realloc (core func $r "realloc"))))
    (core module $m
      (import "" "stream.read"
        (func $stream.read (param i32 i32 i32) (result i32)))
      (func (export "start") (param i32) (result i32)
        (drop
          (call $stream.read (local.get 0) (i32.const 0x100) (i32.const 1)))
        (i32.const 1 (; CALLBACK_CODE_YIELD, stay alive ;)))
      (func (export "cb") (param i32 i32 i32) (result i32)
        (i32.const 1 (; CALLBACK_CODE_YIELD ;))))
    (core instance $i
      (instantiate $m
        (with "" (instance (export "stream.read" (func $stream.read))))))
    (func (export "start") async (param "s" (stream string))
      (canon lift (core func $i "start") async
        (callback (core func $i "cb")))))

  (component $D
    (import "start" (func $start async (param "s" (stream string))))
    (core module $libc
      (memory (export "mem") 1)
      (data (i32.const 0x200) "hi"))
    (core instance $libc (instantiate $libc))
    (type $ST (stream string))
    (core func $start (canon lower (func $start) async))
    (core func $stream.new (canon stream.new $ST))
    (core func $stream.write
      (canon stream.write $ST async (memory (core memory $libc "mem"))))
    (core module $m
      (import "" "mem" (memory 1))
      (import "" "start" (func $start (param i32) (result i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write"
        (func $stream.write (param i32 i32 i32) (result i32)))
      (func (export "run") (result i32)
        (local $s i64) (local $r i32) (local $w i32)
        (local.set $s (call $stream.new))
        (local.set $r (i32.wrap_i64 (local.get $s)))
        (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $s) (i64.const 32))))
        (drop (call $start (local.get $r)))
        (i32.store (i32.const 0x100) (i32.const 0x200))
        (i32.store (i32.const 0x104) (i32.const 2))
        (block $caught
          (try_table (catch_all $caught)
            (drop
              (call $stream.write (local.get $w) (i32.const 0x100)
                (i32.const 1))))
          (return (i32.const 42)))
        ;; the reader's exception leaked into this component
        unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "mem" (memory $libc "mem"))
      (export "start" (func $start))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))))))
    (func (export "run") (result u32) (canon lift (core func $i "run"))))

  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "start" (func $c "start"))))
  (func (export "run") (alias export $d "run")))

(assert_trap (invoke "run") "uncaught exception propagated out of component")

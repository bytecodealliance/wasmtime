;;! component_model_async = true
;;! multi_memory = true

;; `post-return` observes the callee task's context slots.
(component
  (component $A
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "f'") (param i32) (result i32)
        (call $set (i32.const 0xcafe))
        (i32.add (local.get 0) (i32.const 42)))
      (func (export "post") (param i32)
        (if (i32.ne (call $get) (i32.const 0xcafe)) (then unreachable)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'") (post-return (core func $m "post"))))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g'") (result i32)
        (call $set (i32.const 0x1234))
        (call $f' (i32.const 1234)))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 1276))

;; A `context.set` performed by `post-return` belongs to the callee's task
;; and must not leak back out into the caller.
(component
  (component $A
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "set" (func $set (param i32)))
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42)))
      (func (export "post") (param i32)
        (call $set (i32.const 0xbeef)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'") (post-return (core func $m "post"))))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $set (i32.const 0x1234))
        (local.set $r (call $f' (i32.const 1234)))
        (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable))
        (local.get $r))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 1276))

;; The callee's `realloc`, invoked by the adapter to lower the string parameters
;; into the callee's memory, runs with fresh context slots on every call: what
;; one call stores is neither visible to the next call nor to the callee
;; function itself.
(component
  (component $A
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (memory (export "memory") 1)
      (global $bump (mut i32) (i32.const 8))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0x9999))
        (local.set $ret (global.get $bump))
        (global.set $bump (i32.and
          (i32.add (i32.add (global.get $bump) (local.get 3)) (i32.const 7))
          (i32.const -8)))
        (local.get $ret))
      (func (export "f'") (param i32 i32 i32 i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" string) (param "y" string)
      (canon lift (core func $m "f'")
        (memory (core memory $m "memory"))
        (realloc (core func $m "realloc"))))
  )

  (component $B
    (import "f" (func $f (param "x" string) (param "y" string)))
    (core module $Libc
      (memory (export "memory") 1)
      (data (i32.const 16) "hello"))
    (core instance $libc (instantiate $Libc))
    (core func $f' (canon lower (func $f)
      (memory (core memory $libc "memory"))))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32 i32 i32 i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g'") (result i32)
        (call $set (i32.const 0x1234))
        (call $f' (i32.const 16) (i32.const 5) (i32.const 16) (i32.const 5))
        (i32.const 100))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 100))

;; The caller's `realloc`, invoked by the adapter to lower the string result
;; back into the caller's memory, also runs with fresh context slots -- it must
;; not see the caller task's slots.
(component
  (component $A
    (core module $M
      (memory (export "memory") 1)
      (data (i32.const 16) "hello")
      (func (export "f'") (result i32)
        (i32.store (i32.const 8) (i32.const 16))
        (i32.store (i32.const 12) (i32.const 5))
        (i32.const 8))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (result string)
      (canon lift (core func $m "f'") (memory (core memory $m "memory"))))
  )

  (component $B
    (import "f" (func $f (result string)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $Libc
      (import "" "get" (func $get (result i32)))
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (i32.const 64))
    )
    (core instance $libc (instantiate $Libc (with "" (instance
      (export "get" (func $get))))))
    (core func $f' (canon lower (func $f)
      (memory (core memory $libc "memory"))
      (realloc (core func $libc "realloc"))))
    (core module $N
      (import "" "f'" (func $f' (param i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g'") (result i32)
        (call $set (i32.const 0x1234))
        (call $f' (i32.const 8))
        (i32.const 200))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 200))

;; A `context.set` performed by the caller's `realloc` is discarded when it
;; returns rather than clobbering the caller task's slots.
(component
  (component $A
    (core module $M
      (memory (export "memory") 1)
      (data (i32.const 16) "hello")
      (func (export "f'") (result i32)
        (i32.store (i32.const 8) (i32.const 16))
        (i32.store (i32.const 12) (i32.const 5))
        (i32.const 8))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (result string)
      (canon lift (core func $m "f'") (memory (core memory $m "memory"))))
  )

  (component $B
    (import "f" (func $f (result string)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $Libc
      (import "" "set" (func $set (param i32)))
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (call $set (i32.const 0x7777))
        (i32.const 64))
    )
    (core instance $libc (instantiate $Libc (with "" (instance
      (export "set" (func $set))))))
    (core func $f' (canon lower (func $f)
      (memory (core memory $libc "memory"))
      (realloc (core func $libc "realloc"))))
    (core module $N
      (import "" "f'" (func $f' (param i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g'") (result i32)
        (call $set (i32.const 0x1234))
        (call $f' (i32.const 8))
        (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable))
        (i32.const 300))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 300))

;; Similar to above, but permuting async in signatures.
(component
  (component $A
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))

    (core module $Libc
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (memory (export "memory") 1)
      (global $bump (mut i32) (i32.const 16))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        ;; Fresh task for every `realloc`, no matter which adapter drives it.
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0xa1))
        (local.set $ret (global.get $bump))
        (global.set $bump (i32.and
          (i32.add (i32.add (global.get $bump) (local.get 3)) (i32.const 7))
          (i32.const -8)))
        (local.get $ret))
    )
    (core instance $libc (instantiate $Libc (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))

    (core func $task.return (canon task.return (result string)
      (memory (core memory $libc "memory"))))

    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (import "" "task.return" (func $task.return (param i32 i32)))
      (import "" "memory" (memory 1))

      (func (export "f-sync") (param $ptr i32) (param $len i32) (result i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0xc0de))
        (i32.store (i32.const 8) (local.get $ptr))
        (i32.store (i32.const 12) (local.get $len))
        (i32.const 8))
      (func (export "f-sync-post") (param i32)
        (if (i32.ne (call $get) (i32.const 0xc0de)) (then unreachable))
        (call $set (i32.const 0xbad)))

      (func (export "f-async") (param $ptr i32) (param $len i32) (result i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0xc0de))
        (call $task.return (local.get $ptr) (local.get $len))
        (i32.const 0 (; CALLBACK_CODE_EXIT ;)))
      (func (export "f-async-cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))
      (export "task.return" (func $task.return))
      (export "memory" (memory $libc "memory"))))))

    (func (export "sync-lift") async (param "x" string) (result string)
      (canon lift (core func $m "f-sync")
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        (post-return (core func $m "f-sync-post"))))
    (func (export "async-lift") async (param "x" string) (result string)
      (canon lift (core func $m "f-async")
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        async
        (callback (core func $m "f-async-cb"))))
  )

  (component $B
    (import "a" (instance $a
      (export "sync-lift" (func async (param "x" string) (result string)))
      (export "async-lift" (func async (param "x" string) (result string)))))

    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))

    (core module $Libc
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (memory (export "memory") 1)
      (data (i32.const 16) "hello")
      (global $bump (mut i32) (i32.const 64))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        (if (i32.ne (call $get) (i32.const 0)) (then unreachable))
        (call $set (i32.const 0xb1))
        (local.set $ret (global.get $bump))
        (global.set $bump (i32.and
          (i32.add (i32.add (global.get $bump) (local.get 3)) (i32.const 7))
          (i32.const -8)))
        (local.get $ret))
    )
    (core instance $libc (instantiate $Libc (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))

    (core func $sync-to-sync (canon lower (func $a "sync-lift")
      (memory (core memory $libc "memory"))
      (realloc (core func $libc "realloc"))))
    (core func $sync-to-async (canon lower (func $a "async-lift")
      (memory (core memory $libc "memory"))
      (realloc (core func $libc "realloc"))))
    (core func $async-to-sync (canon lower (func $a "sync-lift") async
      (memory (core memory $libc "memory"))
      (realloc (core func $libc "realloc"))))
    (core func $async-to-async (canon lower (func $a "async-lift") async
      (memory (core memory $libc "memory"))
      (realloc (core func $libc "realloc"))))

    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (import "" "sync-to-sync" (func $sync-to-sync (param i32 i32 i32)))
      (import "" "sync-to-async" (func $sync-to-async (param i32 i32 i32)))
      (import "" "async-to-sync" (func $async-to-sync (param i32 i32 i32) (result i32)))
      (import "" "async-to-async" (func $async-to-async (param i32 i32 i32) (result i32)))

      (func (export "sync-to-sync")
        (call $set (i32.const 0x1234))
        (call $sync-to-sync (i32.const 16) (i32.const 5) (i32.const 8))
        (if (i32.ne (call $get) (i32.const 0x1234)) (then unreachable)))

      (func (export "sync-to-async")
        (call $set (i32.const 0x1235))
        (call $sync-to-async (i32.const 16) (i32.const 5) (i32.const 8))
        (if (i32.ne (call $get) (i32.const 0x1235)) (then unreachable)))

      (func (export "async-to-sync")
        (call $set (i32.const 0x1236))
        (if (i32.ne
              (call $async-to-sync (i32.const 16) (i32.const 5) (i32.const 8))
              (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (if (i32.ne (call $get) (i32.const 0x1236)) (then unreachable)))

      (func (export "async-to-async")
        (call $set (i32.const 0x1237))
        (if (i32.ne
              (call $async-to-async (i32.const 16) (i32.const 5) (i32.const 8))
              (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (if (i32.ne (call $get) (i32.const 0x1237)) (then unreachable)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))
      (export "sync-to-sync" (func $sync-to-sync))
      (export "sync-to-async" (func $sync-to-async))
      (export "async-to-sync" (func $async-to-sync))
      (export "async-to-async" (func $async-to-async))))))

    (func (export "sync-to-sync") async (canon lift (core func $m "sync-to-sync")))
    (func (export "sync-to-async") async (canon lift (core func $m "sync-to-async")))
    (func (export "async-to-sync") async (canon lift (core func $m "async-to-sync")))
    (func (export "async-to-async") async (canon lift (core func $m "async-to-async")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "sync-to-sync" (func $b "sync-to-sync"))
  (export "sync-to-async" (func $b "sync-to-async"))
  (export "async-to-sync" (func $b "async-to-sync"))
  (export "async-to-async" (func $b "async-to-async"))
)
(assert_return (invoke "sync-to-sync"))
(assert_return (invoke "sync-to-async"))
(assert_return (invoke "async-to-sync"))
(assert_return (invoke "async-to-async"))

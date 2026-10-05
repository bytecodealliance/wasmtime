;;! component_model_async = true
;;! multi_memory = true

;; post-return makes an async-lowered call and that should trap
(component
  (component $C
    (core module $m
      (import "" "task.return" (func $task.return))
      (func (export "f") (result i32)
        (call $task.return)
        i32.const 0 (; EXIT ;))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core func $task.return (canon task.return))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "cb")))))
  (instance $c (instantiate $C))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (func (export "run") (result i32) i32.const 7)
      (func (export "post") (param i32)
        (drop (call $f))))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))))))
    (func (export "run") (result u32)
      (canon lift (core func $i "run") (post-return (core func $i "post")))))
  (instance $b (instantiate $B (with "f" (func $c "f"))))
  (export "run" (func $b "run")))

(assert_trap (invoke "run") "cannot leave component instance")

;; post-return makes a sync-lowered call to an async callee and that should trap.
(component
  (component $C
    (core module $m
      (import "" "task.return" (func $task.return))
      (func (export "f") (result i32)
        (call $task.return)
        i32.const 0 (; EXIT ;))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core func $task.return (canon task.return))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "cb")))))
  (instance $c (instantiate $C))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run") (result i32) i32.const 7)
      (func (export "post") (param i32)
        (call $f)))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))))))
    (func (export "run") (result u32)
      (canon lift (core func $i "run") (post-return (core func $i "post")))))
  (instance $b (instantiate $B (with "f" (func $c "f"))))
  (export "run" (func $b "run")))

(assert_trap (invoke "run") "cannot leave component instance")

;; Same as above, but the callee yields, so the post-return would block.
(component
  (component $C
    (core module $m
      (import "" "task.return" (func $task.return))
      (func (export "f") (result i32)
        i32.const 1 (; YIELD ;))
      (func (export "cb") (param i32 i32 i32) (result i32)
        (call $task.return)
        i32.const 0 (; EXIT ;)))
    (core func $task.return (canon task.return))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "cb")))))
  (instance $c (instantiate $C))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run") (result i32) i32.const 7)
      (func (export "post") (param i32)
        (call $f)))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))))))
    (func (export "run") (result u32)
      (canon lift (core func $i "run") (post-return (core func $i "post")))))
  (instance $b (instantiate $B (with "f" (func $c "f"))))
  (export "run" (func $b "run")))

(assert_trap (invoke "run") "cannot leave component instance")

;; The caller's `realloc` runs inside the callee's `task.return` (via the
;; fused `async-return` adapter) and attempts an async-lowered call.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core module $m
      (import "" "task.return" (func $task.return (param i32 i32)))
      (import "" "mem" (memory 1))
      (data (i32.const 100) "hi")
      (func (export "g") (result i32)
        (call $task.return (i32.const 100) (i32.const 2))
        i32.const 0 (; EXIT ;))
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

  (component $D
    (core module $m
      (func (export "h") (result i32) i32.const 0 (; EXIT ;))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m))
    (func (export "h") async
      (canon lift (core func $i "h") async (callback (core func $i "cb")))))
  (instance $d (instantiate $D))

  (component $A
    (import "g" (func $g async (result string)))
    (import "h" (func $h async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $h (canon lower (func $h) async))
    (core module $r
      (import "" "h" (func $h (result i32)))
      (global $bump (mut i32) (i32.const 1000))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $r i32)
        (drop (call $h))
        (local.set $r (global.get $bump))
        (global.set $bump (i32.add (global.get $bump) (local.get 3)))
        (local.get $r)))
    (core instance $r (instantiate $r (with "" (instance
      (export "h" (func $h))))))
    (core func $g (canon lower (func $g) async
      (memory (core memory $libc "mem"))
      (realloc (core func $r "realloc"))))
    (core module $m
      (import "" "g" (func $g (param i32) (result i32)))
      (func (export "run") (result i32)
        (call $g (i32.const 16))))
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))))))
    (func (export "run") (result u32) (canon lift (core func $i "run"))))
  (instance $a (instantiate $A
    (with "g" (func $c "g"))
    (with "h" (func $d "h"))))
  (export "run" (func $a "run")))

(assert_trap (invoke "run") "cannot leave component instance")

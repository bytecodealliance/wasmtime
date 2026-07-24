;;! component_model_async = true
;;! component_model_async_stackful = true
;;! gc = true

;; Regression test for https://github.com/bytecodealliance/wasmtime/issues/13676
;;
;; A stackful component-model-async guest task ("holder") allocates a GC
;; reference and holds it in a local across a suspension point. While that fiber
;; is suspended, a sibling task triggers a garbage collection. If we don't trace
;; GC roots in suspended fibers, then we will get GC heap corruption.

(component
  (import "wasmtime" (instance $wasmtime
    (export "gc" (func))
  ))

  ;; The driver subtask: yields control back to the holder (so the holder can
  ;; suspend), then churns the GC heap.
  (component $Driver
    (import "wasmtime" (instance $wasmtime
      (export "gc" (func))
    ))

    (core func $gc (canon lower (func $wasmtime "gc")))
    (core func $yield (canon thread.yield))

    (core module $m
      (import "" "gc" (func $gc))
      (import "" "yield" (func $yield (result i32)))

      (type $s (struct (field i32)))

      (func (export "drive")
        ;; Hand control back to the holder so it can reach `waitable-set.wait`
        ;; and suspend with its GC reference live on its fiber stack.
        (drop (call $yield))

        ;; The holder is now suspended. Collect: this must trace the holder's
        ;; suspended fiber, otherwise its struct is freed here.
        (call $gc)

        ;; Allocate a batch of same-shaped garbage. Under deferred
        ;; reference counting this reuses the just-freed slot (if the bug is
        ;; present) and overwrites it with a different value.
        (call $churn)

        ;; Collect again for good measure (helps relocating collectors expose a
        ;; stale, un-updated pointer).
        (call $gc)
      )

      (func $churn
        (local $i i32)
        (local.set $i (i32.const 128))
        (loop $l
          (drop (struct.new $s (i32.const 0x5678)))
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (br_if $l (local.get $i))
        )
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "gc" (func $gc))
        (export "yield" (func $yield))
      ))
    ))

    (func (export "drive") async (canon lift (core func $i "drive")))
  )
  (instance $driver (instantiate $Driver (with "wasmtime" (instance $wasmtime))))

  ;; The holder task: allocates a GC reference, starts the driver, and waits for
  ;; it -- suspending with the reference live -- then validates the reference
  ;; survived.
  (component $Holder
    (import "drive" (func $drive async))

    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (core func $drive (canon lower (func $drive) async))
    (core func $ws-new (canon waitable-set.new))
    (core func $w-join (canon waitable.join))
    (core func $ws-wait (canon waitable-set.wait (memory (core memory $libc "memory"))))
    (core func $subtask-drop (canon subtask.drop))

    (core module $m
      (import "" "drive" (func $drive (result i32)))
      (import "" "ws-new" (func $ws-new (result i32)))
      (import "" "w-join" (func $w-join (param i32 i32)))
      (import "" "ws-wait" (func $ws-wait (param i32 i32) (result i32)))
      (import "" "subtask-drop" (func $subtask-drop (param i32)))

      (type $s (struct (field i32)))

      (func (export "run")
        (local $s (ref $s))
        (local $ret i32)
        (local $task i32)
        (local $set i32)

        ;; The reference we want to survive the collection that happens while we
        ;; are suspended below.
        (local.set $s (struct.new $s (i32.const 0x1234)))

        ;; Start the driver subtask. It will `thread.yield` almost immediately,
        ;; handing control back here, so it must not have already RETURNED (2).
        (local.set $ret (call $drive))
        (if (i32.eq (i32.and (local.get $ret) (i32.const 0xf)) (i32.const 2))
          (then unreachable))
        (local.set $task (i32.shr_u (local.get $ret) (i32.const 4)))

        ;; Wait for the driver to finish. This suspends our fiber with `$s` live
        ;; across the call, which is exactly the situation the GC must handle.
        (local.set $set (call $ws-new))
        (call $w-join (local.get $task) (local.get $set))
        (drop (call $ws-wait (local.get $set) (i32.const 0)))

        ;; Resumed: the driver ran two collections while we were suspended. If
        ;; our fiber roots were traced, `$s` still reads 0x1234.
        (if (i32.ne (struct.get $s 0 (local.get $s)) (i32.const 0x1234))
          (then unreachable))

        (call $subtask-drop (local.get $task))
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "drive" (func $drive))
        (export "ws-new" (func $ws-new))
        (export "w-join" (func $w-join))
        (export "ws-wait" (func $ws-wait))
        (export "subtask-drop" (func $subtask-drop))
      ))
    ))

    (func (export "run") async (canon lift (core func $i "run")))
  )
  (instance $holder (instantiate $Holder (with "drive" (func $driver "drive"))))

  (export "run" (func $holder "run"))
)

(assert_return (invoke "run"))

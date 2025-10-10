;;! component_model_async = true
;;! component_model_async_stackful = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

;; async lift; no callback
(component
  (core module $m
    (func (export "foo") (param i32) unreachable)
  )
  (core instance $i (instantiate $m))

  (func (export "foo") (param "p1" u32) (result u32)
    (canon lift (core func $i "foo") async)
  )
)

;; async lower -> async lift without callback
(component
  (component $lifter
    (core module $m
      (import "" "task.return" (func $task-return (param i32)))
      (func (export "foo") (param i32) (call $task-return (local.get 0)))
    )
    (core func $task-return (canon task.return (result u32)))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task-return))))
    ))

    (func (export "foo") (param "p1" u32) (result u32)
      (canon lift (core func $i "foo") async)
    )
  )

  (component $lowerer
    (import "a" (func $foo (param "p1" u32) (result u32)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $foo (canon lower (func $foo) async (memory $libc "memory")))
    (core module $m
      (import "libc" "memory" (memory 1))
      (import "" "foo" (func $foo (param i32 i32) (result i32)))
      (func (export "run")
        block
          (call $foo (i32.const 42) (i32.const 1204))
          (i32.eq (i32.load offset=0 (i32.const 1204)) (i32.const 42))
          br_if 0
          unreachable
        end
      )
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance (export "foo" (func $foo))))
    ))
    (func (export "run") (canon lift (core func $i "run")))
  )

  (instance $lifter (instantiate $lifter))
  (instance $lowerer (instantiate $lowerer (with "a" (func $lifter "foo"))))
  (func (export "run") (alias export $lowerer "run"))
)

(assert_return (invoke "run"))

;; sync lower -> async lift without callback
(component
  (component $lifter
    (core module $m
      (import "" "task.return" (func $task-return (param i32)))
      (func (export "foo") (param i32) (call $task-return (local.get 0)))
    )
    (core func $task-return (canon task.return (result u32)))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task-return))))
    ))

    (func (export "foo") (param "p1" u32) (result u32)
      (canon lift (core func $i "foo") async)
    )
  )

  (component $lowerer
    (import "a" (func $foo (param "p1" u32) (result u32)))
    (core func $foo (canon lower (func $foo)))
    (core module $m
      (import "" "foo" (func $foo (param i32) (result i32)))
      (func (export "run")
        block
          (i32.eq (call $foo (i32.const 42)) (i32.const 42))
          br_if 0
          unreachable
        end
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "foo" (func $foo))))
    ))
    (func (export "run") (canon lift (core func $i "run")))
  )

  (instance $lifter (instantiate $lifter))
  (instance $lowerer (instantiate $lowerer (with "a" (func $lifter "foo"))))
  (func (export "run") (alias export $lowerer "run"))
)

(assert_return (invoke "run"))

;; waitable-set.wait
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.wait" (func $waitable-set-wait (param i32 i32) (result i32)))
  )
  (core func $waitable-set-wait (canon waitable-set.wait cancellable (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.wait" (func $waitable-set-wait))))))
)

;; waitable-set.poll
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.poll" (func $waitable-set-poll (param i32 i32) (result i32)))
  )
  (core func $waitable-set-poll (canon waitable-set.poll cancellable (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.poll" (func $waitable-set-poll))))))
)

;; yield
(component
  (core module $m
    (import "" "yield" (func $yield (result i32)))
  )
  (core func $yield (canon thread.yield cancellable))
  (core instance $i (instantiate $m (with "" (instance (export "yield" (func $yield))))))
)

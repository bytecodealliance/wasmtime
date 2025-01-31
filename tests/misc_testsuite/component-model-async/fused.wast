;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

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
          (i32.store offset=0 (i32.const 1200) (i32.const 42))
          (call $foo (i32.const 1200) (i32.const 1204))
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

;; TODO: this requires async support in `wasmtime-wast`:
;;(assert_return (invoke "run"))

;; async lower -> async lift with callback
(component
  (component $lifter
    (core module $m
      (import "" "task.return" (func $task-return (param i32)))
      (func (export "callback") (param i32 i32 i32 i32) (result i32) unreachable)
      (func (export "foo") (param i32) (result i32)
        (call $task-return (local.get 0))
        i32.const 0
      )
    )
    (core func $task-return (canon task.return (result u32)))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task-return))))
    ))

    (func (export "foo") (param "p1" u32) (result u32)
      (canon lift (core func $i "foo") async (callback (func $i "callback")))
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
          (i32.store offset=0 (i32.const 1200) (i32.const 42))
          (call $foo (i32.const 1200) (i32.const 1204))
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

;; TODO: this requires async support in `wasmtime-wast`:
;;(assert_return (invoke "run"))

;; async lower -> sync lift
(component
  (component $lifter
    (core module $m
      (func (export "foo") (param i32) (result i32)
         local.get 0
      )
    )
    (core instance $i (instantiate $m))
    (func (export "foo") (param "p1" u32) (result u32)
      (canon lift (core func $i "foo"))
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
          (i32.store offset=0 (i32.const 1200) (i32.const 42))
          (call $foo (i32.const 1200) (i32.const 1204))
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

;; TODO: this requires async support in `wasmtime-wast`:
;;(assert_return (invoke "run"))

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

;; TODO: this requires async support in `wasmtime-wast`:
;;(assert_return (invoke "run"))

;; sync lower -> async lift with callback
(component
  (component $lifter
    (core module $m
      (import "" "task.return" (func $task-return (param i32)))
      (func (export "callback") (param i32 i32 i32 i32) (result i32) unreachable)
      (func (export "foo") (param i32) (result i32)
        (call $task-return (local.get 0))
        i32.const 0
      )
    )
    (core func $task-return (canon task.return (result u32)))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task-return))))
    ))

    (func (export "foo") (param "p1" u32) (result u32)
      (canon lift (core func $i "foo") async (callback (func $i "callback")))
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

;; TODO: this requires async support in `wasmtime-wast`:
;;(assert_return (invoke "run"))

;;! component_model_async = true
;;! reference_types = true

;; An example component where a sync lower calls an async lift. The async lift
;; is an early-return but then keeps going afterwards for a late exit.

(component
  (component $A
    (core module $m
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "callback") (param i32 i32 i32) (result i32)
        i32.const 0 ;; EXIT
      )
      (func (export "foo") (param i32) (result i32)
        (call $task.return (local.get 0))
        i32.const 1 ;; YIELD
      )
    )
    (core func $task.return (canon task.return (result u32)))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task.return))))
    ))

    (func (export "foo") (param "p1" u32) (result u32)
      (canon lift (core func $i "foo") async (callback (func $i "callback")))
    )
  )

  (component $B
    (import "a" (func $foo (param "p1" u32) (result u32)))
    (core func $foo (canon lower (func $foo)))
    (core module $m
      (import "" "foo" (func $foo (param i32) (result i32)))
      (func (export "run")
        (i32.ne (call $foo (i32.const 42)) (i32.const 42))
        if unreachable end
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "foo" (func $foo))))
    ))
    (func (export "run") (canon lift (core func $i "run")))
  )

  (instance $A (instantiate $A))
  (instance $B (instantiate $B (with "a" (func $A "foo"))))
  (func (export "run") (alias export $B "run"))
)

(assert_return (invoke "run"))
(assert_return (invoke "run"))

;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

;; synchronous future.read; sync lift
(component
  (component $child
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (type $future (future))
    (core func $read (canon future.read $future (memory $libc "memory")))

    (core module $m
      (import "" "read" (func $read (param i32 i32) (result i32)))

      (func (export "run") (param $future i32)
        (call $read (local.get $future) (i32.const 0))
        drop
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "read" (func $read))
      ))
    ))
    (func (export "run") (param "x" $future)
      (canon lift (core func $i "run")))
  )
  (instance $child (instantiate $child))

  (type $future (future))
  (core func $new (canon future.new $future))
  (core func $child-run (canon lower (func $child "run")))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "child-run" (func $child-run (param i32)))

    (func (export "run")
      (call $child-run (i32.wrap_i64 (call $new)))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "child-run" (func $child-run))
    ))
  ))

  (func (export "run")
    (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "synchronous stream and future reads not yet supported")

;; asynchronous future.read; sync lift
(component
  (component $child
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (type $future (future))
    (core func $read (canon future.read $future (memory $libc "memory") async))

    (core module $m
      (import "" "read" (func $read (param i32 i32) (result i32)))

      (func (export "run") (param $future i32)
        (call $read (local.get $future) (i32.const 0))
        drop
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "read" (func $read))
      ))
    ))
    (func (export "run") (param "x" $future)
      (canon lift (core func $i "run")))
  )
  (instance $child (instantiate $child))

  (type $future (future))
  (core func $new (canon future.new $future))
  (core func $child-run (canon lower (func $child "run")))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "child-run" (func $child-run (param i32)))

    (func (export "run")
      (call $child-run (i32.wrap_i64 (call $new)))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "child-run" (func $child-run))
    ))
  ))

  (func (export "run")
    (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))

;; synchronous future.read; async lift
(component
  (component $child
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (type $future (future))
    (core func $read (canon future.read $future (memory $libc "memory")))

    (core module $m
      (import "" "read" (func $read (param i32 i32) (result i32)))

      (func (export "run") (param $future i32) (result i32)
        (call $read (local.get $future) (i32.const 0))
        drop
        i32.const 0 ;; TODO
      )

      (func (export "cb") (param i32 i32 i32) (result i32)
        i32.const 0) ;; TODO
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "read" (func $read))
      ))
    ))
    (func (export "run") (param "x" $future)
      (canon lift (core func $i "run") async (callback (func $i "cb"))))
  )
  (instance $child (instantiate $child))

  (type $future (future))
  (core func $new (canon future.new $future))
  (core func $child-run (canon lower (func $child "run")))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "child-run" (func $child-run (param i32)))

    (func (export "run")
      (call $child-run (i32.wrap_i64 (call $new)))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "child-run" (func $child-run))
    ))
  ))

  (func (export "run")
    (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "synchronous stream and future reads not yet supported")

;; asynchronous future.read; async lift
(component
  (component $child
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (type $future (future))
    (core func $read (canon future.read $future (memory $libc "memory") async))
    (core func $return (canon task.return))

    (core module $m
      (import "" "read" (func $read (param i32 i32) (result i32)))
      (import "" "return" (func $return))

      (func (export "run") (param $future i32) (result i32)
        (call $read (local.get $future) (i32.const 0))
        drop
        call $return
        i32.const 0
      )

      (func (export "cb") (param i32 i32 i32) (result i32)
        unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "read" (func $read))
        (export "return" (func $return))
      ))
    ))
    (func (export "run") (param "x" $future)
      (canon lift (core func $i "run") async (callback (func $i "cb"))))
  )
  (instance $child (instantiate $child))

  (type $future (future))
  (core func $new (canon future.new $future))
  (core func $child-run (canon lower (func $child "run")))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "child-run" (func $child-run (param i32)))

    (func (export "run")
      (call $child-run (i32.wrap_i64 (call $new)))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "child-run" (func $child-run))
    ))
  ))

  (func (export "run")
    (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))


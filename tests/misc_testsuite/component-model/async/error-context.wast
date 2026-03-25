;;! component_model_async = true
;;! component_model_error_context = true

;; error-context.new
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "error-context.new" (func $error-context-new (param i32 i32) (result i32)))
  )
  (core func $error-context-new (canon error-context.new (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "error-context.new" (func $error-context-new))))))
)

;; error-context.debug-message
(component
  (core module $libc
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    (memory (export "memory") 1)
  )
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "error-context.debug-message" (func $error-context-debug-message (param i32 i32)))
  )
  (core func $error-context-debug-message (canon error-context.debug-message (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core instance $i (instantiate $m (with "" (instance (export "error-context.debug-message" (func $error-context-debug-message))))))
)

;; error-context.drop
(component
  (core module $m
    (import "" "error-context.drop" (func $error-context-drop (param i32)))
  )
  (core func $error-context-drop (canon error-context.drop))
  (core instance $i (instantiate $m (with "" (instance (export "error-context.drop" (func $error-context-drop))))))
)

;; Test edge-case behavior of `error-context.debug-message`.
(component definition $A
  (core module $libc
    (memory (export "memory") 1)
    (global $bump (mut i32) (i32.const 100))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $ret i32)
      (if (local.get 0) (then unreachable))
      (if (local.get 1) (then unreachable))
      (if (i32.ne (local.get 2) (i32.const 1)) (then unreachable))
      (local.set $ret (global.get $bump))
      (global.set $bump (i32.add (global.get $bump) (local.get 3)))
      (local.get $ret)
    )
  )
  (core instance $libc (instantiate $libc))

  (core module $Core
    (import "" "mem" (memory 1))
    (import "" "error-context.new" (func $error-context.new (param i32 i32) (result i32)))
    (import "" "error-context.debug-message" (func $error-context.debug-message (param i32 i32)))
    (import "" "error-context.drop" (func $error-context.drop (param i32)))

    (func (export "run") (param $dst i32)
      (local $handle i32)
      (i32.store8 (i32.const 0) (i32.const 0x61)) ;; 'a'
      (local.set $handle (call $error-context.new (i32.const 0) (i32.const 1)))
      (call $error-context.debug-message (local.get $handle) (local.get $dst))
      (call $error-context.drop (local.get $handle))
    )
  )

  (core func $error-context.new (canon error-context.new (memory $libc "memory")))
  (core func $error-context.debug-message (canon error-context.debug-message (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core func $error-context.drop (canon error-context.drop))

  (core instance $core (instantiate $Core (with "" (instance
    (export "mem" (memory $libc "memory"))
    (export "error-context.new" (func $error-context.new))
    (export "error-context.debug-message" (func $error-context.debug-message))
    (export "error-context.drop" (func $error-context.drop))
  ))))

  (func (export "run") (param "p" u32) (canon lift (core func $core "run")))
)

(component instance $A $A)
(assert_return (invoke "run" (u32.const 65528)))
(assert_trap (invoke "run" (u32.const 65532)) "invalid debug message pointer")

;;! component_model_error_context = true

(component definition $Tester
  (core module $Memory
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
  )
  (core instance $memory (instantiate $Memory))

  (core func $error-context.new (canon error-context.new (memory $memory "mem")))
  (core func $error-context.debug-message
    (canon error-context.debug-message
      (memory $memory "mem")
      (realloc (func $memory "realloc"))))
  (core func $error-context.drop (canon error-context.drop))

  (core module $DM
    (import "" "mem" (memory 1))
    (import "" "error-context.new" (func $error-context.new (param i32 i32) (result i32)))
    (import "" "error-context.debug-message" (func $error-context.debug-message (param i32 i32)))
    (import "" "error-context.drop" (func $error-context.drop (param i32)))

    (func (export "noop"))
    (func (export "trap-calling-error-context-new") (call $error-context.new (i32.const 0) (i32.const 0)) drop)
    (func (export "trap-calling-error-context-debug-message") (call $error-context.debug-message (i32.const 0) (i32.const 0)))
    (func (export "trap-calling-error-context-drop") (call $error-context.drop (i32.const 0)))
  )
  (core instance $dm (instantiate $DM (with "" (instance
    (export "mem" (memory $memory "mem"))
    (export "error-context.new" (func $error-context.new))
    (export "error-context.debug-message" (func $error-context.debug-message))
    (export "error-context.drop" (func $error-context.drop))
  ))))
  (func (export "trap-calling-error-context-new")
    (canon lift (core func $dm "noop")
      (post-return (func $dm "trap-calling-error-context-new"))))
  (func (export "trap-calling-error-context-debug-message")
    (canon lift (core func $dm "noop")
      (post-return (func $dm "trap-calling-error-context-debug-message"))))
  (func (export "trap-calling-error-context-drop")
    (canon lift (core func $dm "noop")
      (post-return (func $dm "trap-calling-error-context-drop"))))
)

(component instance $i0 $Tester)
(assert_trap (invoke "trap-calling-error-context-new") "cannot leave component instance")
(component instance $i1 $Tester)
(assert_trap (invoke "trap-calling-error-context-debug-message") "cannot leave component instance")
(component instance $i2 $Tester)
(assert_trap (invoke "trap-calling-error-context-drop") "cannot leave component instance")

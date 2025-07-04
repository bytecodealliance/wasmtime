(component
  (import "sleep" (func $sleep))

  (component $A
    (import "run-stackless" (func $run_stackless))
    (import "run-stackful" (func $run_stackful))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (core func $task_return (canon task.return))
    (core func $waitable_set_new (canon waitable-set.new))
    (core func $waitable_set_wait (canon waitable-set.wait (memory $libc "memory")))
    (core func $waitable_join (canon waitable.join))

    (canon lower (func $run_stackless) async (core func $run_stackless))
    (canon lower (func $run_stackful) async (core func $run_stackful))

    (core module $m
      (import "" "memory" (memory 1))
      (import "" "task.return" (func $task_return))
      (import "" "waitable-set.new" (func $waitable_set_new (result i32)))
      (import "" "waitable-set.wait" (func $waitable_set_wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable_join (param i32 i32)))
      (import "" "run-stackless" (func $run_stackless (result i32)))
      (import "" "run-stackful" (func $run_stackful (result i32)))

      (global $set (mut i32) (i32.const 0))
      (global $call (mut i32) (i32.const 0))

      (func (export "run-stackless") (result i32)
        (local $ret i32)
        (local $status i32)
        (local.set $ret (call $run_stackless))
        (local.set $status (i32.and (local.get $ret) (i32.const 0xF)))
        (global.set $call (i32.shr_u (local.get $ret) (i32.const 4)))
        (if (result i32) (i32.eq (i32.const 1 (; STARTED ;)) (local.get $status))
          (then
            (global.set $set (call $waitable_set_new))
            (call $waitable_join (global.get $call) (global.get $set))
            (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $set) (i32.const 4))))
          (else
            (if (result i32) (i32.eq (i32.const 2 (; RETURNED ;)) (local.get $status))
               (then
                 (call $task_return)
                 (i32.const 0 (; EXIT ;)))
               (else unreachable)))))

      (func (export "run-stackful")
        (local $ret i32)
        (local $set i32)
        (local $status i32)  
        (local $call i32)
        (local $event0 i32)
        (local $event1 i32)
        (local $event2 i32)
        (local.set $ret (call $run_stackful))
        (local.set $status (i32.and (local.get $ret) (i32.const 0xF)))
        (local.set $call (i32.shr_u (local.get $ret) (i32.const 4)))
        (if (i32.eq (i32.const 1 (; STARTED ;)) (local.get $status))
          (then
            (local.set $set (call $waitable_set_new))
            (call $waitable_join (local.get $call) (local.get $set))
            (local.set $event0 (call $waitable_set_wait (local.get $set) (i32.const 0)))
            (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $event0))
              (then unreachable))
            (local.set $event1 (i32.load (i32.const 0)))
            (local.set $event2 (i32.load (i32.const 4)))
            (if (i32.ne (local.get $call) (local.get $event1))
              (then unreachable))
            (if (i32.ne (i32.const 2 (; RETURNED ;)) (local.get $event2))
              (then unreachable)))
          (else
            (if (i32.ne (i32.const 2 (; RETURNED ;)) (local.get $status))
               (then unreachable))))
        (call $task_return))

      (func (export "cb") (param $event0 i32) (param $event1 i32) (param $event2 i32) (result i32)
        (local $status i32)
        (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $event0))
          (then unreachable))
        (call $task_return)
        (i32.const 0 (; EXIT ;))))

    (core instance $i (instantiate $m
      (with "" (instance
        (export "memory" (memory $libc "memory"))
        (export "task.return" (func $task_return))
        (export "waitable-set.new" (func $waitable_set_new))
        (export "waitable-set.wait" (func $waitable_set_wait))
        (export "waitable.join" (func $waitable_join))
        (export "run-stackless" (func $run_stackless))
        (export "run-stackful" (func $run_stackful))))))

    (func (export "run-stackless") (canon lift (core func $i "run-stackless") async (callback (func $i "cb"))))
    (func (export "run-stackful") (canon lift (core func $i "run-stackful") async)))

  (instance $a (instantiate $A
    (with "run-stackless" (func $sleep))
    (with "run-stackful" (func $sleep))))
  (instance $b (instantiate $A
    (with "run-stackless" (func $a "run-stackless"))
    (with "run-stackful" (func $a "run-stackful"))))
  (func (export "run-stackless") (alias export $a "run-stackless"))
  (func (export "run-stackful") (alias export $a "run-stackful"))
  (func (export "run-stackless-stackless") (alias export $b "run-stackless"))
  (func (export "run-stackful-stackful") (alias export $b "run-stackful")))

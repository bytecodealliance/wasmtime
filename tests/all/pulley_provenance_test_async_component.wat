(component
  (import "return-slowly" (func $return-slowly async))

  (component $A
    (import "run-stackless" (func $run_stackless async))
    (import "run-stackful" (func $run_stackful async))
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
        (i32.const 0 (; EXIT ;)))
    )

    (core instance $i (instantiate $m
      (with "" (instance
        (export "memory" (memory $libc "memory"))
        (export "task.return" (func $task_return))
        (export "waitable-set.new" (func $waitable_set_new))
        (export "waitable-set.wait" (func $waitable_set_wait))
        (export "waitable.join" (func $waitable_join))
        (export "run-stackless" (func $run_stackless))
        (export "run-stackful" (func $run_stackful))))))

    (func (export "run-stackless") async (canon lift (core func $i "run-stackless") async (callback (func $i "cb"))))
    (func (export "run-stackful") async (canon lift (core func $i "run-stackful") async))
  )

  (instance $a (instantiate $A
    (with "run-stackless" (func $return-slowly))
    (with "run-stackful" (func $return-slowly))))
  (instance $b (instantiate $A
    (with "run-stackless" (func $a "run-stackless"))
    (with "run-stackful" (func $a "run-stackful"))))
  (func (export "run-stackless") (alias export $a "run-stackless"))
  (func (export "run-stackful") (alias export $a "run-stackful"))
  (func (export "run-stackless-stackless") (alias export $b "run-stackless"))
  (func (export "run-stackful-stackful") (alias export $b "run-stackful"))


  ;; Test which exercises an intra-component stream read/write to ensure that
  ;; there's no Miri violations while doing this.
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (core module $ics
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
    (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
    (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
    (import "" "mem" (memory 1))

    (global $r (mut i32) (i32.const 0))
    (global $w (mut i32) (i32.const 0))

    (func (export "run")
      (local $t64 i64)

      (local.set $t64 (call $stream.new))
      (global.set $r (i32.wrap_i64 (local.get $t64)))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $t64) (i64.const 32))))

      (call $transfer (i32.const 100) (i32.const 200) (i32.const 100))
      (call $transfer (i32.const 200) (i32.const 100) (i32.const 100))
      (call $transfer (i32.const 150) (i32.const 151) (i32.const 100))
      (call $transfer (i32.const 151) (i32.const 150) (i32.const 100))

      (call $stream.drop-readable (global.get $r))
      (call $stream.drop-writable (global.get $w))
    )

    (func $transfer (param $src i32) (param $dst i32) (param $len i32)
      (local $ws i32)

      (call $stream.read
        (global.get $r)
        (local.get $dst)
        (local.get $len))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $stream.write
        (global.get $w)
        (local.get $src)
        (local.get $len))
      (i32.shl (local.get $len) (i32.const 4)) ;; (len << 4) | COMPLETED
                                               ;;   where COMPLETED==0
      i32.ne
      if unreachable end

      ;; Reap the readable status on the `$r` handle now.
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (global.get $r) (local.get $ws))
      (call $waitable-set.wait (local.get $ws) (i32.const 0))
      i32.const 2 ;; EVENT_STREAM_READ
      i32.ne
      if unreachable end
      (call $waitable.join (global.get $r) (i32.const 0))
      (call $waitable-set.drop (local.get $ws))
    )
  )
  (type $s (stream u8))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "mem")))
  (core func $stream.write (canon stream.write $s async (memory $libc "mem")))
  (core func $stream.drop-readable (canon stream.drop-readable $s))
  (core func $stream.drop-writable (canon stream.drop-writable $s))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait (canon waitable-set.wait (memory $libc "mem")))
  (core func $waitable-set.drop (canon waitable-set.drop))
  (core instance $ics (instantiate $ics
    (with "" (instance
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "stream.drop-writable" (func $stream.drop-writable))
      (export "mem" (memory $libc "mem"))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "waitable-set.drop" (func $waitable-set.drop))
    ))
  ))
  (func (export "intra-component-stream") async
    (canon lift (core func $ics "run")))
)

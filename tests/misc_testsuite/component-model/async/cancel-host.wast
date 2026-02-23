;;! component_model_async = true

;; This test starts a host subtask that never returns which takes a borrow.
;;
;; When cancelling that subtask it should correctly yield the borrow back to the
;; guest and allow the guest to destroy the resource.
(component
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[method]resource1.never-return" (func async (param "self" (borrow $r))))
  ))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (import "" "new" (func $new (param i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "drop-subtask" (func $drop-subtask (param i32)))
    (import "" "drop-resource" (func $drop-resource (param i32)))

    (func (export "run")
      (local $handle i32)
      (local $subtask i32)

      ;; Create an owned resource
      (call $new (i32.const 100))
      local.set $handle

      ;; Call async function with a borrow of the resource.
      ;; This returns STARTED (1) | (subtask_id << 4).
      (call $f (local.get $handle))
      local.tee $subtask

      ;; Check status is STARTED (lower 4 bits = 1)
      i32.const 0xf
      i32.and
      i32.const 1 ;; STARTED
      i32.ne
      if unreachable end

      ;; Extract subtask id
      local.get $subtask
      i32.const 4
      i32.shr_u
      local.set $subtask

      ;; Cancel the subtask — should release the borrow
      (call $cancel (local.get $subtask))
      i32.const 4 ;; RETURN_CANCELLED
      i32.ne
      if unreachable end

      ;; Drop the subtask
      (call $drop-subtask (local.get $subtask))

      ;; Drop the owned resource
      (call $drop-resource (local.get $handle))
    )
  )
  (alias export $host "resource1" (type $r))
  (core func $f (canon lower (func $host "[method]resource1.never-return") async))
  (core func $new (canon lower (func $host "[constructor]resource1")))
  (core func $cancel (canon subtask.cancel))
  (core func $drop-subtask (canon subtask.drop))
  (core func $drop-resource (canon resource.drop $r))
  (core instance $i (instantiate $m
      (with "" (instance
          (export "f" (func $f))
          (export "new" (func $new))
          (export "cancel" (func $cancel))
          (export "drop-subtask" (func $drop-subtask))
          (export "drop-resource" (func $drop-resource))
      ))
  ))

  (func (export "f") async
      (canon lift (core func $i "run")))
)

(assert_return (invoke "f"))

;; This test starts two subtasks and waits for one to complete. Cancelling the
;; second one should then work correctly. Historically this triggered a panic
;; in Wasmtime.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))
  ))

  (core module $Mem (memory (export "mem") 1))
  (core instance $mem (instantiate $Mem))

  (core module $m
    (import "" "slow" (func $slow (param i32) (result i32)))
    (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
    (import "" "subtask.drop" (func $subtask.drop (param i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
    (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
    (func (export "run")
      (local $s1 i32) (local $s2 i32) (local $ws i32) (local $tmp i32)

      ;; start `slow` twice
      (local.set $s1 (call $start-slow))
      (local.set $s2 (call $start-slow))

      ;; Wait for slow to complete via waitable-set
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get $s1) (local.get $ws))
      (drop (call $waitable-set.wait (local.get $ws) (i32.const 104)))

      ;; first task returned, and if the second task is cancelled then nothing
      ;; bad should happen...
      ;;
      ;; Note that this cancellation may indicate that the host task returned,
      ;; or it may return it was cancelled, that's up to the host.
      (call $subtask.cancel (local.get $s2))
      drop

      (call $subtask.drop (local.get $s2))
      (call $subtask.drop (local.get $s1))

      ;; Clean up the waitable-set.
      (call $waitable-set.drop (local.get $ws))
    )

    (func $start-slow (result i32)
      (local $tmp i32)

      ;; Start slow, expect STARTED
      (call $slow (i32.const 100))
      local.tee $tmp
      i32.const 0xf
      i32.and
      i32.const 1 ;; STARTED
      i32.ne
      if unreachable end
      local.get $tmp
      i32.const 4
      i32.shr_u
    )
  )
  (core func $slow (canon lower (func $host "return-two-slowly") async (memory $mem "mem")))
  (core func $subtask.cancel (canon subtask.cancel))
  (core func $subtask.drop (canon subtask.drop))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait (canon waitable-set.wait (memory $mem "mem")))
  (core func $waitable-set.drop (canon waitable-set.drop))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "slow" (func $slow))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "subtask.drop" (func $subtask.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "waitable-set.drop" (func $waitable-set.drop))
    ))
  ))

  (func (export "run") async
    (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))


;; Similar to the above test, but asserts that `subtask.cancel` can't be called
;; twice on the same host task.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))
  ))

  (core module $Mem (memory (export "mem") 1))
  (core instance $mem (instantiate $Mem))

  (core module $m
    (import "" "slow" (func $slow (param i32) (result i32)))
    (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
    (import "" "subtask.drop" (func $subtask.drop (param i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
    (import "" "thread.yield" (func $thread.yield (result i32)))
    (func (export "run")
      (local $s1 i32) (local $s2 i32) (local $ws i32) (local $tmp i32)

      ;; start `slow` twice
      (local.set $s1 (call $start-slow))
      (local.set $s2 (call $start-slow))

      ;; Wait for slow to complete via waitable-set
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get $s1) (local.get $ws))
      (drop (call $waitable-set.wait (local.get $ws) (i32.const 104)))

      ;; first task returned, and if the second task is cancelled then nothing
      ;; bad should happen...
      ;;
      ;; Note that this cancellation may indicate that the host task returned,
      ;; or it may return it was cancelled, that's up to the host.
      (call $subtask.cancel (local.get $s2))
      drop

      ;; let the host do something else for a moment
      (drop (call $thread.yield))

      ;; calling cancel again on this task should trap since we already received
      ;; a terminal status code from above.
      (call $subtask.cancel (local.get $s2))
      unreachable
    )

    (func $start-slow (result i32)
      (local $tmp i32)

      ;; Start slow, expect STARTED
      (call $slow (i32.const 100))
      local.tee $tmp
      i32.const 0xf
      i32.and
      i32.const 1 ;; STARTED
      i32.ne
      if unreachable end
      local.get $tmp
      i32.const 4
      i32.shr_u
    )
  )
  (core func $slow (canon lower (func $host "return-two-slowly") async (memory $mem "mem")))
  (core func $subtask.cancel (canon subtask.cancel))
  (core func $subtask.drop (canon subtask.drop))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait (canon waitable-set.wait (memory $mem "mem")))
  (core func $thread.yield (canon thread.yield))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "slow" (func $slow))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "subtask.drop" (func $subtask.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "thread.yield" (func $thread.yield))
    ))
  ))

  (func (export "run") async
    (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "`subtask.cancel` called after terminal status delivered")

;; This test covers a historical bug in Wasmtime where cancelled host tasks
;; could keep running in a sort of zombie state which would clobber other tasks.
;;
;; Here two tasks are started, the first completes, the second is cancelled,
;; another is started/waited on. It's then asserted that the cancelled
;; task's side effects are not visible.
(component
  (import "host" (instance $host
    (export "echo-slowly" (func async (param "val" u32) (result u32)))
  ))

  (core module $Mem (memory (export "mem") 1))
  (core instance $mem (instantiate $Mem))

  (core module $m
    (import "" "mem" (memory 1))
    ;; echo: (val, retptr) → status|handle
    (import "" "echo" (func $echo (param i32 i32) (result i32)))
    (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
    (import "" "subtask.drop" (func $subtask.drop (param i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
    (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))

    (func (export "run")
      (local $e0 i32) (local $e111 i32) (local $e222 i32)
      (local $e111-returned i32)

      ;; Start echo(0,retptr=0) first
      (local.set $e0 (call $start-echo (i32.const 0) (i32.const 0)))

      ;; Start echo(111,retptr=100)
      (local.set $e111 (call $start-echo (i32.const 111) (i32.const 100)))

      ;; wait for $e0 to complete
      (call $wait-for (local.get $e0))

      ;; Cancel/drop echo(111)
      (local.set $e111-returned
        (i32.ne
          (call $subtask.cancel (local.get $e111))
          (i32.const 4) ;; RETURN_CANCELLED=4
        ))
      (call $subtask.drop (local.get $e111))

      ;; Start echo(222,retptr=200)
      (local.set $e222 (call $start-echo (i32.const 222) (i32.const 200)))

      ;; Wait for echo(222).
      (call $wait-for (local.get $e222))

      ;; retptr=100: should be 0 or 111 depending on if it returned
      local.get $e111-returned
      if
        (call $assert-eq (i32.load (i32.const 100)) (i32.const 111))
      else
        (call $assert-eq (i32.load (i32.const 100)) (i32.const 0))
      end
      ;; retptr=200: should be 222.
      (call $assert-eq (i32.load (i32.const 200)) (i32.const 222))

      ;; Cleanup.
      (call $subtask.drop (local.get $e222))
      (call $subtask.drop (local.get $e0))
    )

    (func $assert-eq (param i32 i32)
      (local.get 0)
      (local.get 1)
      i32.ne
      if unreachable end
    )

    ;; start a call to `echo(local.get 0, local.get 1)`
    (func $start-echo (param i32 i32) (result i32)
      (local $tmp i32)
      (call $echo (local.get 0) (local.get 1))
      local.set $tmp
      (call $assert-eq
        (i32.and (local.get $tmp) (i32.const 0xf))
        (i32.const 0x1))
      (i32.shr_u (local.get $tmp) (i32.const 4))
    )

    ;; wait for the waitable identified by local 0
    (func $wait-for (param i32)
      (local $ws i32)

      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get 0) (local.get $ws))
      (call $assert-eq
        (call $waitable-set.wait (local.get $ws) (i32.const 500))
        (i32.const 1) ;; EVENT_SUBTASK
      )
      (call $waitable.join (local.get 0) (i32.const 0))

      (call $assert-eq
        (i32.load (i32.const 500))
        (local.get 0))

      (call $waitable-set.drop (local.get $ws))
    )
  )
  (core func $echo (canon lower (func $host "echo-slowly") async (memory $mem "mem")))
  (core func $subtask.cancel (canon subtask.cancel))
  (core func $subtask.drop (canon subtask.drop))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait (canon waitable-set.wait (memory $mem "mem")))
  (core func $waitable-set.drop (canon waitable-set.drop))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "mem" (memory $mem "mem"))
      (export "echo" (func $echo))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "subtask.drop" (func $subtask.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "waitable-set.drop" (func $waitable-set.drop))
    ))
  ))

  (func (export "run") async (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))

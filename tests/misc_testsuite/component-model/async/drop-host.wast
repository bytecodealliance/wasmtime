;;! component_model_async = true

;; Can't drop a host subtask which has completed on the host but hasn't been
;; notified to the guest that it's resolved.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))
  ))

  (core module $Mem (memory (export "mem") 1))
  (core instance $mem (instantiate $Mem))

  (core module $m
    (import "" "slow" (func $slow (param i32) (result i32)))
    (import "" "subtask.drop" (func $subtask.drop (param i32)))
    (import "" "thread.yield" (func $thread.yield (result i32)))
    (func (export "run")
      (local $s1 i32)

      (local.set $s1 (call $start-slow))
      (drop (call $thread.yield))
      (call $subtask.drop (local.get $s1))
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
  (core func $thread.yield (canon thread.yield))
  (core func $subtask.drop (canon subtask.drop))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "slow" (func $slow))
      (export "thread.yield" (func $thread.yield))
      (export "subtask.drop" (func $subtask.drop))
    ))
  ))

  (func (export "run") async
    (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "cannot drop a subtask which has not yet resolved")

;;! component_model_async = true
;;! component_model_threading = true

;; A thread blocked in waitable-set.wait is still running for the purposes of
;; thread.resume-later and thread.suspend-then-resume.  In particular, neither
;; operation may wake it without a waitable event.
(component definition $C
  (core module $libc
    (memory (export "mem") 1)
    (table (export "table") 1 funcref))
  (core instance $libc (instantiate $libc))
  (core type $start (func (param i32)))
  (core func $thread.new-indirect
    (canon thread.new-indirect $start (core table $libc "table")))
  (core func $thread.yield-then-resume (canon thread.yield-then-resume))
  (core func $thread.resume-later (canon thread.resume-later))
  (core func $thread.suspend-then-resume (canon thread.suspend-then-resume))
  (type $stream (stream u8))
  (core func $stream.new (canon stream.new $stream))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait
    (canon waitable-set.wait (memory (core memory $libc "mem"))))

  (core module $m
    (import "libc" "mem" (memory 1))
    (import "libc" "table" (table $table 1 funcref))
    (import "" "thread.new-indirect" (func $new (param i32 i32) (result i32)))
    (import "" "thread.yield-then-resume" (func $yield-to (param i32) (result i32)))
    (import "" "thread.resume-later" (func $resume-later (param i32)))
    (import "" "thread.suspend-then-resume" (func $suspend-to (param i32) (result i32)))
    (import "" "stream.new" (func $stream-new (result i64)))
    (import "" "waitable-set.new" (func $set-new (result i32)))
    (import "" "waitable.join" (func $join (param i32 i32)))
    (import "" "waitable-set.wait" (func $wait (param i32 i32) (result i32)))

    (func $waiter (param i32)
      (local $ends i64) (local $set i32)
      (local.set $ends (call $stream-new))
      (local.set $set (call $set-new))
      (call $join (i32.wrap_i64 (local.get $ends)) (local.get $set))
      ;; The writable end stays open, so there is no event to wake the waiter.
      (drop (call $wait (local.get $set) (i32.const 0))))
    (elem (table $table) (i32.const 0) func $waiter)

    (func $start-waiter (result i32)
      (local $id i32)
      (local.set $id (call $new (i32.const 0) (i32.const 0)))
      (drop (call $yield-to (local.get $id)))
      (local.get $id))

    (func (export "resume-later-waiter")
      (call $resume-later (call $start-waiter)))
    (func (export "suspend-then-resume-waiter")
      (drop (call $suspend-to (call $start-waiter)))))

  (core instance $i (instantiate $m
    (with "libc" (instance $libc))
    (with "" (instance
      (export "thread.new-indirect" (func $thread.new-indirect))
      (export "thread.yield-then-resume" (func $thread.yield-then-resume))
      (export "thread.resume-later" (func $thread.resume-later))
      (export "thread.suspend-then-resume" (func $thread.suspend-then-resume))
      (export "stream.new" (func $stream.new))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.wait" (func $waitable-set.wait))))))
  (func (export "resume-later-waiter") async
    (canon lift (core func $i "resume-later-waiter")))
  (func (export "suspend-then-resume-waiter") async
    (canon lift (core func $i "suspend-then-resume-waiter"))))

(component instance $resume-later $C)
(assert_trap (invoke "resume-later-waiter")
  "cannot resume thread which is not suspended")
(component instance $suspend-then-resume $C)
(assert_trap (invoke "suspend-then-resume-waiter")
  "cannot resume thread which is not suspended")

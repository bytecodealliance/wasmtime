;;! component_model_async = true
;;! component_model_more_async_builtins = true

;; `$B` calls `$A`'s `f`, which never finishes, and requests its cancellation
;; with an async `subtask.cancel`, which blocks since `f` ignores the request.
;; Requesting cancellation of the same subtask a second time traps.
(component
  (component $A
    (core module $m
      (func (export "f") (result i32)
        (i32.const 1 (; YIELD ;)))
      (func (export "f-cb") (param i32 i32 i32) (result i32)
        (i32.const 1 (; YIELD ;))))
    (core instance $i (instantiate $m))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb")))))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core func $subtask.cancel (canon subtask.cancel async))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (func (export "run") (result i32)
        (local $ret i32) (local $subtask i32)
        (local.set $ret (call $f))
        (if (i32.ne (i32.and (local.get $ret) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
        (if (i32.ne (call $subtask.cancel (local.get $subtask))
              (i32.const -1 (; BLOCKED ;)))
          (then unreachable))
        (drop (call $subtask.cancel (local.get $subtask)))
        unreachable)
      (func (export "run-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "subtask.cancel" (func $subtask.cancel))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "run-cb")))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
)

(assert_trap (invoke "run") "`subtask.cancel` called after terminal status delivered")

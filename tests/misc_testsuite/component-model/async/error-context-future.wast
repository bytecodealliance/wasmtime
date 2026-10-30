;;! component_model_async = true
;;! component_model_error_context = true

;; `$B` sends an error-context to `$A` through a future, and `$A` drops the
;; copy it received. `$B` still owns its own handle, so dropping it should
;; succeed.
(component
  (component $A
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (type $FT (future error-context))
    (core func $future.read
      (canon future.read $FT async (memory (core memory $libc "memory"))))
    (core func $error-context.drop (canon error-context.drop))
    (core module $m
      (import "libc" "memory" (memory 1))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "error-context.drop" (func $error-context.drop (param i32)))
      (func (export "take") (param $f i32)
        (if (i32.ne (call $future.read (local.get $f) (i32.const 0))
              (i32.const 0 (; COMPLETED ;)))
          (then unreachable))
        (call $error-context.drop (i32.load (i32.const 0))))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "future.read" (func $future.read))
        (export "error-context.drop" (func $error-context.drop))))))
    (func (export "take") (param "f" $FT) (canon lift (core func $i "take"))))
  (instance $a (instantiate $A))

  (component $B
    (import "take" (func $take (param "f" (future error-context))))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (type $FT (future error-context))
    (core func $take (canon lower (func $take)))
    (core func $future.new (canon future.new $FT))
    (core func $future.write
      (canon future.write $FT async (memory (core memory $libc "memory"))))
    (core func $error-context.new
      (canon error-context.new (memory (core memory $libc "memory"))))
    (core func $error-context.drop (canon error-context.drop))
    (core module $m
      (import "libc" "memory" (memory 1))
      (import "" "take" (func $take (param i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "error-context.new" (func $error-context.new (param i32 i32) (result i32)))
      (import "" "error-context.drop" (func $error-context.drop (param i32)))
      (func (export "run")
        (local $futures i64) (local $e i32)
        (local.set $e (call $error-context.new (i32.const 0) (i32.const 0)))
        (local.set $futures (call $future.new))
        (i32.store (i32.const 0) (local.get $e))
        (if (i32.ne (call $future.write
                      (i32.wrap_i64 (i64.shr_u (local.get $futures) (i64.const 32)))
                      (i32.const 0))
              (i32.const -1 (; BLOCKED ;)))
          (then unreachable))
        (call $take (i32.wrap_i64 (local.get $futures)))
        (call $error-context.drop (local.get $e)))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "take" (func $take))
        (export "future.new" (func $future.new))
        (export "future.write" (func $future.write))
        (export "error-context.new" (func $error-context.new))
        (export "error-context.drop" (func $error-context.drop))))))
    (func (export "run") async (canon lift (core func $i "run"))))
  (instance $b (instantiate $B (with "take" (func $a "take"))))
  (export "run" (func $b "run"))
)

(assert_return (invoke "run"))

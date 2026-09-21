;;! tail_call = true

;; Tail-call through canon lower, then resize the argument area in the provider.
(component
  (component $provider
    (core module $m
      (func $final
        (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
        (result i32)
        local.get 0)
      (func (export "f")
        (param i32 i32 i32 i32 i32 i32 i32 i32 i32)
        (result i32)
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        local.get 4
        local.get 5
        local.get 6
        local.get 7
        local.get 8
        i32.const 10
        i32.const 11
        i32.const 12
        i32.const 13
        return_call $final))
    (core instance $i (instantiate $m))
    (func (export "f")
      (param "a" u32)
      (param "b" u32)
      (param "c" u32)
      (param "d" u32)
      (param "e" u32)
      (param "f" u32)
      (param "g" u32)
      (param "h" u32)
      (param "i" u32)
      (result u32)
      (canon lift (core func $i "f"))))

  (instance $provider-instance (instantiate $provider))

  (component $consumer
    (import "f" (func $f
      (param "a" u32)
      (param "b" u32)
      (param "c" u32)
      (param "d" u32)
      (param "e" u32)
      (param "f" u32)
      (param "g" u32)
      (param "h" u32)
      (param "i" u32)
      (result u32)))
    (core func $lowered-f (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f
        (param i32 i32 i32 i32 i32 i32 i32 i32 i32)
        (result i32)))
      (func (export "run") (param i32) (result i32)
        local.get 0
        i32.const 2
        i32.const 3
        i32.const 4
        i32.const 5
        i32.const 6
        i32.const 7
        i32.const 8
        i32.const 9
        return_call $f))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $lowered-f))))))
    (func (export "run") (param "x" u32) (result u32)
      (canon lift (core func $i "run"))))

  (instance $consumer-instance (instantiate $consumer
    (with "f" (func $provider-instance "f"))))
  (export "run" (func $consumer-instance "run")))

(assert_return (invoke "run" (u32.const 42)) (u32.const 42))

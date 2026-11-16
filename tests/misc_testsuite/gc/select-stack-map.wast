;;! gc = true

;; The result of a `select` with a GC reference type must be in stack maps.
(module
  (type $pair (struct (field (mut i32))))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (param $cond i32) (result i32)
    ;; The select result stays on the operand stack, never in a local.
    (select (result (ref null $pair))
      (struct.new $pair (i32.const 111))
      (ref.null $pair)
      (local.get $cond))

    ;; This call is a safepoint, and the select result is live across it. If
    ;; it's missing from the stack map, the GC frees the struct and the next
    ;; allocation may reuse its memory.
    (call $gc)
    (drop (struct.new $pair (i32.const 222)))

    (struct.get $pair 0))
)

(assert_return (invoke "test" (i32.const 1)) (i32.const 111))

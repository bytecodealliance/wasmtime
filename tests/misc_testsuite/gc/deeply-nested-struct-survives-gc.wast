;;! gc = true

(module
  (import "wasmtime" "gc" (func $gc))
  (type $node (struct (field i32) (field (mut (ref null $node)))))

  (func (export "test") (result i32)
    (local $n1 (ref null $node))
    (local $n2 (ref null $node))
    (local $n3 (ref null $node))
    (local $n4 (ref null $node))
    ;; Build chain: n4 -> n3 -> n2 -> n1
    (local.set $n1 (struct.new $node (i32.const 1) (ref.null $node)))
    (local.set $n2 (struct.new $node (i32.const 2) (local.get $n1)))
    (call $gc)
    (local.set $n3 (struct.new $node (i32.const 3) (local.get $n2)))
    (call $gc)
    (local.set $n4 (struct.new $node (i32.const 4) (local.get $n3)))
    (call $gc)
    ;; Traverse: n4.next.next.next.val should be 1
    (struct.get $node 0
      (struct.get $node 1
        (struct.get $node 1
          (struct.get $node 1
            (local.get $n4)
          )
        )
      )
    )
  )
)

(assert_return (invoke "test") (i32.const 1))

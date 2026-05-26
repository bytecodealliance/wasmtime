;;! gc = true

;; Test: Many allocations forcing many GC cycles and heap growths.
;;
;; The copying collector must properly handle repeated flip/copy/resize cycles.

(module
  (type $node (struct (field (ref null $node)) (field i32)))

  (import "wasmtime" "gc" (func $gc))

  ;; Build a linked list of n nodes, each storing its index
  (func $build (param $n i32) (result (ref null $node))
    (local $head (ref null $node))
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $head
          (struct.new $node (local.get $head) (local.get $i))
        )
        (call $gc)
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)
      )
    )
    (local.get $head)
  )

  ;; Sum all values in the linked list
  (func $sum (param $head (ref null $node)) (result i32)
    (local $total i32)
    (local $cur (ref null $node))
    (local.set $cur (local.get $head))
    (block $done
      (loop $loop
        (br_if $done (ref.is_null (local.get $cur)))
        (local.set $total
          (i32.add (local.get $total)
                   (struct.get $node 1 (local.get $cur)))
        )
        (local.set $cur (struct.get $node 0 (local.get $cur)))
        (br $loop)
      )
    )
    (local.get $total)
  )

  ;; Build a list of 20 nodes, force GC, then sum. Expected: 0+1+...+19 = 190
  (func (export "test-linked-list") (result i32)
    (local $list (ref null $node))
    (local.set $list (call $build (i32.const 50)))
    (call $gc)
    (call $sum (local.get $list))
  )
)

(assert_return (invoke "test-linked-list")
               ;; 0+1+2+...+49 = 49*50/2 = 1225
               (i32.const 1225))

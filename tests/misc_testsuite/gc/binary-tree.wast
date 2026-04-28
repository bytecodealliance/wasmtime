;;! gc = true

(module
  (type $tree (struct
    (field (ref null $tree))  ;; left
    (field (ref null $tree))  ;; right
    (field i32)               ;; value
  ))

  (import "wasmtime" "gc" (func $gc))

  ;; Build a complete binary tree of depth n, with values 1..2^n-1
  (func $build (param $depth i32) (param $val i32) (result (ref null $tree))
    (local $left (ref null $tree))
    (local $right (ref null $tree))
    (if (result (ref null $tree)) (i32.le_s (local.get $depth) (i32.const 0))
      (then (ref.null $tree))
      (else
        (local.set $left
          (call $build
            (i32.sub (local.get $depth) (i32.const 1))
            (i32.mul (local.get $val) (i32.const 2))
          )
        )
        (local.set $right
          (call $build
            (i32.sub (local.get $depth) (i32.const 1))
            (i32.add (i32.mul (local.get $val) (i32.const 2)) (i32.const 1))
          )
        )
        (struct.new $tree
          (local.get $left)
          (local.get $right)
          (local.get $val)
        )
        (call $gc)
      )
    )
  )

  ;; Sum all values in the tree
  (func $sum (param $t (ref null $tree)) (result i32)
    (if (result i32) (ref.is_null (local.get $t))
      (then (i32.const 0))
      (else
        (i32.add
          (struct.get $tree 2 (local.get $t))
          (i32.add
            (call $sum (struct.get $tree 0 (local.get $t)))
            (call $sum (struct.get $tree 1 (local.get $t)))
          )
        )
      )
    )
  )

  ;; Build tree of depth 5 (31 nodes), sum all values
  ;; Values: 1,2,3,...,31 -> sum = 31*32/2 = 496
  (func (export "test") (result i32)
    (call $sum (call $build (i32.const 5) (i32.const 1)))
  )
)

(assert_return (invoke "test") (i32.const 496))

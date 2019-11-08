(module (func))

(module
  (func (export "if-without-result") (param i32) (param i32) (result i32)
    (if
      (i32.eq
        (local.get 0)
        (local.get 1)
      )
      (then (unreachable))
    )

    (local.get 0)
  )
)

(assert_return (invoke "if-without-result" (i32.const 2) (i32.const 3)) (i32.const 2))

(module
  (func (export "block") (param i32) (param i32) (result i32)
    (block (result i32)
      local.get 0
    )
  )
)

(assert_return (invoke "block" (i32.const 10) (i32.const 20)) (i32.const 10))

(module
  (func (export "br_block") (param i32) (param i32) (result i32)
    local.get 1
    (block (result i32)
      local.get 0
      local.get 0
      br 0
      unreachable
    )
    i32.add
  )
)

(assert_return (invoke "br_block" (i32.const 5) (i32.const 7)) (i32.const 12))

;; Tests discarding values on the value stack, while
;; carrying over the result using a conditional branch.
(module
  (func (export "brif_block") (param i32) (param i32) (result i32)
    local.get 1
    (block (result i32)
      local.get 0
      local.get 0
      br_if 0
      unreachable
    )
    i32.add
  )
)

(assert_return (invoke "brif_block" (i32.const 5) (i32.const 7)) (i32.const 12))

;; Tests that br_if keeps values in the case if the branch
;; hasn't been taken.
(module
  (func (export "brif_block_passthru") (param i32) (param i32) (result i32)
    (block (result i32)
      local.get 1
      local.get 0
      br_if 0
      local.get 1
      i32.add
    )
  )
)

(assert_return (invoke "brif_block_passthru" (i32.const 0) (i32.const 3)) (i32.const 6))

(module
  (func (export "i32.div_s") (param i32) (param i32) (result i32)
    (i32.div_s (local.get 0) (local.get 1))
  )
)

(module
  (func (export "br_table") (param $i i32) (result i32)
    (return
      (block $2 (result i32)
        (i32.add (i32.const 10)
          (block $1 (result i32)
            (i32.add (i32.const 100)
              (block $0 (result i32)
                (i32.add (i32.const 1000)
                  (block $default (result i32)
                    (br_table $0 $1 $2 $default
                      (i32.mul (i32.const 2) (local.get $i))
                      (i32.and (i32.const 3) (local.get $i))
                    )
                  )
                )
              )
            )
          )
        )
      )
    )
  )
)

(assert_return (invoke "br_table" (i32.const 0)) (i32.const 110))
(assert_return (invoke "br_table" (i32.const 1)) (i32.const 12))
(assert_return (invoke "br_table" (i32.const 2)) (i32.const 4))
(assert_return (invoke "br_table" (i32.const 3)) (i32.const 1116))
(assert_return (invoke "br_table" (i32.const 4)) (i32.const 118))
(assert_return (invoke "br_table" (i32.const 5)) (i32.const 20))
(assert_return (invoke "br_table" (i32.const 6)) (i32.const 12))
(assert_return (invoke "br_table" (i32.const 7)) (i32.const 1124))
(assert_return (invoke "br_table" (i32.const 8)) (i32.const 126))

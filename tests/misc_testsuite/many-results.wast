(module
  (func (export "f")
    (result
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32
    )

    i32.const 0
    i32.const 1
    i32.const 2
    i32.const 3
    i32.const 4
    i32.const 5
    i32.const 6
    i32.const 7
    i32.const 8
    i32.const 9
    i32.const 10
    i32.const 11
    i32.const 12
    i32.const 13
    i32.const 14
    i32.const 15
    i32.const 16
  )
)

(assert_return (invoke "f")
  (i32.const 0)
  (i32.const 1)
  (i32.const 2)
  (i32.const 3)
  (i32.const 4)
  (i32.const 5)
  (i32.const 6)
  (i32.const 7)
  (i32.const 8)
  (i32.const 9)
  (i32.const 10)
  (i32.const 11)
  (i32.const 12)
  (i32.const 13)
  (i32.const 14)
  (i32.const 15)
  (i32.const 16)
)

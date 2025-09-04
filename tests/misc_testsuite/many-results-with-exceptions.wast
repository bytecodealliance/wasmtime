;;! exceptions = true

(module
  (func (export "f")
    (result
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32
    )

    (block $h
      (try_table (catch_all $h)
        call $f_callee
        return
      )
    )
    unreachable
  )
  (func $f_callee
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

  (func (export "f2")
    (param
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32
    )
    (result
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32
    )

    (block $h
      (try_table (catch_all $h)
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        local.get 4
        local.get 5
        local.get 6
        local.get 7
        local.get 8
        local.get 9
        local.get 10
        local.get 11
        local.get 12
        local.get 13
        local.get 14
        local.get 15
        local.get 16

        call $f2_callee
        return
      )
    )
    unreachable
  )
  (func $f2_callee
    (param
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32
    )
    (result
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32 i32 i32 i32
      i32
    )

    local.get 0
    local.get 1
    local.get 2
    local.get 3
    local.get 4
    local.get 5
    local.get 6
    local.get 7
    local.get 8
    local.get 9
    local.get 10
    local.get 11
    local.get 12
    local.get 13
    local.get 14
    local.get 15
    local.get 16
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

(assert_return (invoke "f2"
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

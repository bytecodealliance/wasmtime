(module
  (type (;0;) (func (result v128 v128 v128)))

  (func $main1 (type 0) (result v128 v128 v128)
    call $main1
    i8x16.add
    call $main1
    i8x16.ge_u
    i16x8.ne
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 87
    select
    unreachable
    i32.const 0
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
  )
  (export "main1" (func $main1))

  (func $main2 (type 0) (result v128 v128 v128)
    call $main2
    i8x16.add
    call $main2
    i8x16.ge_u
    i16x8.ne
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 87
    select (result v128)
    unreachable
    i32.const 0
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
  )
  (export "main2" (func $main2))
)

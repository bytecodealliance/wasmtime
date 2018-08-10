(module
  (type $indirect_sig (func (param i64) (result i64)))

  (func $assert (param i32)
    (block $ok
      (br_if $ok
        (get_local 0)
      )
      (unreachable)
    )
  )

  (func $plus_1 (param i64) (result i64)
    get_local 0
    i64.const 1
    i64.add
  )
  (func $minus_1 (param i64) (result i64)
    get_local 0
    i64.const 1
    i64.sub
  )

  (func $main 
    (call $call_indirect
      (i32.const 0)
      (i64.const 2)
    )
    (call $call_indirect
      (i32.const 1)
      (i64.const 0)
    )
  )

  (func $call_indirect (param $func i32) (param $expected i64)
    (call $assert
      (i64.eq
        (call_indirect (type $indirect_sig)
          (i64.const 1)
          (get_local $func)
        )
        (get_local $expected)
      )
    )
  )
  (start $main)
  
  (table 2 2 anyfunc)
  (elem (i32.const 0) $plus_1 $minus_1)
)

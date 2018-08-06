(module
  (memory 1)
  (func $assert (param i32)
    (block $ok
      (br_if $ok
        (get_local 0)
      )
      (unreachable)
    )
  )
  (func $main (local i32)
    (call $assert
      (i32.eq
        (grow_memory (i32.const 1))
        (i32.const 1)
      )
    )
    (call $assert
      (i32.eq
        (current_memory)
        (i32.const 2)
      )
    )
  )
  (start $main)
  (data (i32.const 0) "\04\03\02\01")
)


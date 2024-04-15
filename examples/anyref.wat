(module
  (table $table (export "table") 10 anyref)

  (global $global (export "global") (mut anyref) (ref.null any))

  (func (export "take_anyref") (param anyref)
    nop
  )

  (func (export "return_anyref") (result anyref)
    i32.const 42
    ref.i31
  )
)

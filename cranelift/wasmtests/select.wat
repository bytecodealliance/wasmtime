(module
  (func $untyped-select (result i32)
  	i32.const 42
  	i32.const 24
  	i32.const 1
  	select)

  (func $typed-select-1 (result anyref)
  	ref.null
  	ref.null
  	i32.const 1
  	select (result anyref))

  (func $typed-select-2 (param anyref) (result anyref)
    ref.null
    local.get 0
    i32.const 1
    select (result anyref))
)

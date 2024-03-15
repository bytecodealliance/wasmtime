(module
  (func $untyped-select (result i32)
  	i32.const 42
  	i32.const 24
  	i32.const 1
  	select)

  (func $typed-select-1 (result externref)
  	ref.null extern
  	ref.null extern
  	i32.const 1
  	select (result externref))

  (func $typed-select-2 (param externref) (result externref)
    ref.null extern
    local.get 0
    i32.const 1
    select (result externref))
)

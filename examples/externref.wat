(module
  (table $table (export "table") 10 externref)

  (global $global (export "global") (mut externref) (ref.null extern))

  (func (export "func") (param externref) (result externref)
    local.get 0
  )
)

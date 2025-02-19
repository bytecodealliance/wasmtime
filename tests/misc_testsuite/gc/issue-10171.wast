;;! gc = true

(module
  (type $tree (struct (field $left anyref)
                      (field $right anyref)))
  (type $s (struct))

  (func (export "f") (result i32)
    struct.new $s
    ref.null i31
    struct.new $tree
    i32.const 2
    ref.i31
    struct.new $tree
    struct.get $tree $left
    ref.cast (ref null $tree)
    struct.get $tree $left
    ref.test (ref null $s)
  )
)

(assert_return (invoke "f") (i32.const 1))

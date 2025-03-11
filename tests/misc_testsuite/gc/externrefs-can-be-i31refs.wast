;;! gc = true

(module
  (type (struct (field externref)))
  (func (export "")
    i32.const 0x7fffffff
    ref.i31
    extern.convert_any
    struct.new 0
    drop
  )
)

(assert_return (invoke ""))

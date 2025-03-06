;;! gc = true
;;! simd = true

(module
  (type $s (struct (field v128)))
  (func (export "alloc")
    struct.new_default $s
    drop
  )
)

(assert_return (invoke "alloc"))

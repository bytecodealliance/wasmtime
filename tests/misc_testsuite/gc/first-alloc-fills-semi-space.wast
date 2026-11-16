;;! gc = true

(module
  (type $a (array (mut i8)))
  (func (export "alloc") (param i32) (result i32)
    (array.len (array.new_default $a (local.get 0)))))

(assert_return (invoke "alloc" (i32.const 32740)) (i32.const 32740))

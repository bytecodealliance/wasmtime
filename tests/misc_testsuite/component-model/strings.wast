;; Returning an unaligned utf16 string is invalid
(component definition $A
  (core module $m
    (memory (export "m") 1)
    (func (export "f") (result i32)
      (i32.store (i32.const 4) (i32.const 1))
      (i32.store (i32.const 8) (i32.const 0))
      i32.const 4
    )
  )
  (core instance $m (instantiate $m))
  (func (export "f1") (result string)
    (canon lift (core func $m "f") (memory $m "m") string-encoding=utf16))
  (func (export "f2") (result string)
    (canon lift (core func $m "f") (memory $m "m") string-encoding=latin1+utf16))

)
(component instance $A $A)
(assert_trap (invoke "f1") "string pointer not aligned to 2")
(component instance $A $A)
(assert_trap (invoke "f2") "string pointer not aligned to 2")

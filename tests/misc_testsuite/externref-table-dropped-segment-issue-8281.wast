;;! reference_types = true

(module
  (table $t 0 0 externref)

  (func (export "f1")
    (i32.const 0)
    (i32.const 0)
    (i32.const 0)
    (table.init $t $declared)
  )

  (func (export "f2")
    (i32.const 0)
    (i32.const 0)
    (i32.const 0)
    (table.init $t $passive)

    (elem.drop $passive)

    (i32.const 0)
    (i32.const 0)
    (i32.const 0)
    (table.init $t $passive)
  )

  (func (export "f3")
    (i32.const 0)
    (i32.const 0)
    (i32.const 0)
    (table.init $t $active)
  )

  (elem $declared declare externref)
  (elem $passive externref)
  (elem $active (i32.const 0) externref)
)

(assert_return (invoke "f1"))
(assert_return (invoke "f2"))
(assert_return (invoke "f3"))

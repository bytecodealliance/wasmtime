;;! gc = true
;;! bulk_memory = true

(module
  (import "wasmtime" "gc" (func $gc))
  (type $s (struct (field i32)))
  (table $t 10 anyref)

  ;; Passive element segment with a GC struct
  (elem $e anyref (struct.new $s (i32.const 42)))

  ;; Copy passive element into table
  (func (export "init")
      (table.init $t $e (i32.const 0) (i32.const 0) (i32.const 1))
  )

  ;; Read the struct from the table and return its field value
  (func (export "get_field") (result i32)
      (struct.get $s 0
          (ref.cast (ref $s)
              (table.get $t (i32.const 0))
          )
      )
  )

  (export "gc" (func $gc))
)

(assert_return (invoke "gc"))
(assert_return (invoke "init"))
(assert_return (invoke "get_field") (i32.const 42))

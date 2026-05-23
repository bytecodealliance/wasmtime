;;! gc = true
;;! bulk_memory = true

(module
  (elem externref (item (extern.convert_any (ref.i31 (i32.const 1)))))
)

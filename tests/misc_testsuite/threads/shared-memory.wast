;;! threads = true

;; Two instances of one module importing the same shared memory see a write
;; made through a third instance.
(module $Mem
  (memory (export "shared") 1 5 shared)
)

(module $Writer
  (import "Mem" "shared" (memory 1 5 shared))
  (func (export "write") (i32.store (i32.const 0) (i32.const 42)))
)
(invoke $Writer "write")

(module definition $Reader
  (import "Mem" "shared" (memory 1 5 shared))
  (func (export "first-word") (result i32) (i32.load (i32.const 0)))
)
(module instance $Reader1 $Reader)
(module instance $Reader2 $Reader)
(assert_return (invoke $Reader1 "first-word") (i32.const 42))
(assert_return (invoke $Reader2 "first-word") (i32.const 42))

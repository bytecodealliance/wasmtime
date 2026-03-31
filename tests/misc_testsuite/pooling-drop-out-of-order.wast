;; Small test case to create a module in one thread, drop another module in
;; another thread, then drop the first module in the original thread. This
;; historically exposed an issue with MPK and using striped indices correctly
;; when purging modules from the pooling allocator.

(module
  (memory 1)
  (data (i32.const 0) "\2a\00\00\00")
  (func (export "load") (result i32)
    i32.const 0
    i32.load))

(assert_return (invoke "load") (i32.const 42))

(thread $t1 (module (memory 1)))
(wait $t1)

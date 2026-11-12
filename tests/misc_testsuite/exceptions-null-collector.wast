;;! exceptions = true
;;! flags = "-C collector=null"

;; Exceptions work with the null collector, both when caught in wasm and when
;; they escape to the host.
(module
  (tag $e (param i32))

  (func $throw (export "throw")
    (throw $e (i32.const 42)))

  (func (export "catch") (result i32)
    (block $h (result i32)
      (try_table (catch $e $h)
        (call $throw))
      unreachable)))

(assert_exception (invoke "throw"))
(assert_return (invoke "catch") (i32.const 42))

(module)

(thread $t1
  (module $A
    (memory 10 100)
    (func (export "grow")
      (drop (memory.grow (i32.const 10)))))
  (invoke "grow")
)
(wait $t1)

(module $B
  (memory 5 100)
  (func (export "read_oob") (result i32)
    (i32.load (i32.const 983040))))
(assert_trap (invoke "read_oob") "out of bounds memory access")

(module definition (func (export "f")))
(module instance)

(invoke "f")
(assert_return (invoke "f"))

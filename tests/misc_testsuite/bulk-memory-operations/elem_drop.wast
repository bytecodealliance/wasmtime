(module
  (table 1 1 funcref)
  (elem (i32.const 0) funcref (ref.func 0))
  (func (export "elem.drop non-passive element")
    (elem.drop 0)))

(invoke "elem.drop non-passive element")

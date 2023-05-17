(module
  (memory (export "mem") 1 1)

  (func (export "load_i32") (param $a i32) (result i32)
    local.get $a
    i32.load)

  (func (export "store_i32") (param $a i32) (param $b i32)
    local.get $a
    local.get $b
    i32.store)
)

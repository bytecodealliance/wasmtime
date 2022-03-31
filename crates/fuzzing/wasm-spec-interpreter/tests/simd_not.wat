(module
  (func (export "simd_not") (param $a v128) (result v128)
    local.get $a
    v128.not))

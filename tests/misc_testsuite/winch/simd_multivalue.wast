;; test that swapping the parameters results in swapped return values
(module (func (export "f") (param v128) (param v128) (result v128) (result v128) (local.get 1) (local.get 0)))
(assert_return (invoke "f" (v128.const i64x2 2 1) (v128.const i64x2 1 2)) (v128.const i64x2 1 2) (v128.const i64x2 2 1))
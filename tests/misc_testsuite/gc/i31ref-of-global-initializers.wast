(module $env
  (global (export "g1") i32 (i32.const 42))
  (global (export "g2") i32 (i32.const 99))
)
(register "env")

(module $i31ref_of_global_const_expr_and_tables
  (global $g1 (import "env" "g1") i32)
  (global $g2 (import "env" "g2") i32)

  (table $t 3 3 (ref i31) (ref.i31 (global.get $g1)))
  (elem (table $t) (i32.const 2) (ref i31) (ref.i31 (global.get $g2)))

  (func (export "get") (param i32) (result i32)
    (i31.get_u (local.get 0) (table.get $t))
  )
)

(assert_return (invoke "get" (i32.const 0)) (i32.const 42))
(assert_return (invoke "get" (i32.const 1)) (i32.const 42))
(assert_return (invoke "get" (i32.const 2)) (i32.const 99))

(module $i31ref_of_global_const_expr_and_globals
  (global $g1 (import "env" "g1") i32)
  (global $g2 i31ref (ref.i31 (global.get $g1)))
  (func (export "get") (result i32)
    (i31.get_u (global.get $g2))
  )
)

(assert_return (invoke "get") (i32.const 42))

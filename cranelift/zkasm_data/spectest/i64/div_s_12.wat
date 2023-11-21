(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const -7
	i64.const 3
	i64.div_s
	i64.const -2
	call $assert_eq)
 (start $main))

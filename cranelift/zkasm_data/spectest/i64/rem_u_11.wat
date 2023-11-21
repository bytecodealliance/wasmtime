(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const -5
	i64.const -2
	i64.rem_u
	i64.const -5
	call $assert_eq)
 (start $main))

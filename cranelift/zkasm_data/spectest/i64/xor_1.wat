(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 1
	i64.const 0
	i64.xor
	i64.const 1
	call $assert_eq)
 (start $main))

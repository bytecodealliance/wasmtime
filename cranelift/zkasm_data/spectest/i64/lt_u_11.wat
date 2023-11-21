(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	i64.const 0x8000000000000000
	i64.const -1
	i64.lt_u
	i32.const 1
	call $assert_eq)
 (start $main))

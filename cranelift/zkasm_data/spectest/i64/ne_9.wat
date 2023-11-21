(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	i64.const 0x8000000000000000
	i64.const 0
	i64.ne
	i32.const 1
	call $assert_eq)
 (start $main))

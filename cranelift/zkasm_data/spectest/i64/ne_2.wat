(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	i64.const 1
	i64.const 1
	i64.ne
	i32.const 0
	call $assert_eq)
 (start $main))

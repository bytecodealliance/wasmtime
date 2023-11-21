(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	i64.const 0x8000000000000000
	i64.eqz
	i32.const 0
	call $assert_eq)
 (start $main))

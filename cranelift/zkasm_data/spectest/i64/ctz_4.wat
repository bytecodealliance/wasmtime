(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x00010000
	i64.ctz
	i64.const 16
	call $assert_eq)
 (start $main))

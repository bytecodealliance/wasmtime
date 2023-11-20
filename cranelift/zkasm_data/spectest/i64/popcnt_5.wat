(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0x7fffffffffffffff
	i64.popcnt
	i64.const 63
	call $assert_eq)
 (start $main))

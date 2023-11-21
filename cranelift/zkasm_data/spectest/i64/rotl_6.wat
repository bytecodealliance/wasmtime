(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0xfe000000dc000000
	i64.const 4
	i64.rotl
	i64.const 0xe000000dc000000f
	call $assert_eq)
 (start $main))

(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0xf0f0ffff
	i64.const 0xfffff0f0
	i64.and
	i64.const 0xf0f0f0f0
	call $assert_eq)
 (start $main))

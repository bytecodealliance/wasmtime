(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 2
	i64.clz
	i64.const 62
	call $assert_eq)
 (start $main))

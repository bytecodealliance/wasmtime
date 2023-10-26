(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	i32.const 1999999999
	i32.const 64516129
	i32.div_s
	i32.const 31
	call $assert_eq)
 (start $main))

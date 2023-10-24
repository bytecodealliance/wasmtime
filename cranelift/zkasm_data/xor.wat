(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	i32.const 2
	i32.const 2
	i32.xor
	i32.const 0
	call $assert_eq)
 (start $main))

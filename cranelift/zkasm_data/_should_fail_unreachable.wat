(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	unreachable
	call $assert_eq)
 (start $main))

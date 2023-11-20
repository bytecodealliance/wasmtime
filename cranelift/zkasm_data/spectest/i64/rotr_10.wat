(module
 (import "env" "assert_eq" (func $assert_eq (param i64) (param i64)))
 (func $main
	i64.const 0xabcd7294ef567809
	i64.const 0xffffffffffffffed
	i64.rotr
	i64.const 0x94a77ab3c04d5e6b
	call $assert_eq)
 (start $main))

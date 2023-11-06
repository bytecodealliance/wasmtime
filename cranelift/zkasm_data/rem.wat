(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
    i32.const 5
    i32.const 2
    i32.rem_s
    i32.const 1
    call $assert_eq
    i32.const 8
    i32.const 3
    i32.rem_s
    i32.const 2
    call $assert_eq)
 (start $main))

(module
 (import "env" "assert_eq" (func $assert_eq (param i32) (param i32)))
 (func $main
	(local $counter i32)
	(local.set $counter (i32.const 0))
	(block
	 (loop
		(local.set $counter
		 (i32.add
			(local.get $counter)
			(i32.const 1)))
		(br_if 1
		 (i32.eq
			(local.get $counter)
			(i32.const 10)
		 )
		)
		(br 0)
	 )
	)
	(local.get $counter)
	(i32.const 10)
	call $assert_eq)
(start $main))

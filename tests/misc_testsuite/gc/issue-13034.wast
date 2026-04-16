;;! gc = true

(module
 (type $i32_array (array (mut i32)))

 (global $arr (mut (ref null $i32_array)) (ref.null $i32_array))

 (func (export "run")
   (global.set $arr
     (array.new $i32_array (i32.const 0) (i32.const 1073741817))
   )
 )
)

(assert_trap (invoke "run") "allocation size too large")

;;! stack_switching = true

;; smoke test for switching: Only a single switch to a cotinuation created with
;; cont.new.
(module

 (type $ft0 (func))
 (type $ct0 (cont $ft0))

 (type $ft1 (func (param (ref $ct0))))
 (type $ct1 (cont $ft1))

 (func $print (import "spectest" "print_i32") (param i32))
 (tag $t)


 (func $f
   (cont.new $ct1 (ref.func $g))
   (switch $ct1 $t)
 )
 (elem declare func $f)

 (func $g (type $ft1)
   (call $print (i32.const 123))
 )
 (elem declare func $g)

 (func $entry (export "entry") (result i32)
   (cont.new $ct0 (ref.func $f))
   (resume $ct0 (on $t switch))
   (i32.const 0)
 )
)

(assert_return (invoke "entry" ) (i32.const 0))

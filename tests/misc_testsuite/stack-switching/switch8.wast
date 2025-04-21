;;! stack_switching = true

;; try to switch to an already consumed continuation
(module
 (type $ft0 (func))
 (type $ct0 (cont $ft0))

 (type $ft1 (func (param (ref null $ct0))))
 (type $ct1 (cont $ft1))

 (func $print (import "spectest" "print_i32") (param i32))
 (tag $t)


 (func $f
   (local $c (ref $ct1))
   (ref.null $ct0) ;; argument to $g
   (cont.new $ct1 (ref.func $g))
   (local.tee $c)
   (resume $ct1)

   ;; this should fail, we already used the continuation
   (local.get $c)
   (switch $ct1 $t)
 )
 (elem declare func $f)

 (func $g (type $ft1))
 (elem declare func $g)

 (func $entry (export "entry")
   (cont.new $ct0 (ref.func $f))
   (resume $ct0 (on $t switch))
 )
)
(assert_trap (invoke "entry") "continuation already consumed")

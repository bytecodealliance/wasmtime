;;! stack_switching = true

;; switch past a suspend handler for same tag
(module

 (type $ft0 (func (result i32)))
 (type $ct0 (cont $ft0))

 (type $ft1 (func (param i32) (result i32)))
 (type $ct1 (cont $ft1))

 (type $ft2 (func (param (ref $ct0)) (result i32)))
 (type $ct2 (cont $ft2))

 (tag $t (result i32))

 (func $f (result i32)
   (block $handler (result (ref $ct1))
     (cont.new $ct0 (ref.func $g))
     (resume $ct0 (on $t $handler))
     ;; $g will switch, we won't come back here
     (unreachable)
   )
   ;; we will not suspend
   (unreachable)
 )
 (elem declare func $f)

 (func $g (result i32)
   (cont.new $ct2 (ref.func $h))
   (switch $ct2 $t)
   ;; we won't come back here
   (unreachable)
 )
 (elem declare func $g)

 (func $h (type $ft2)
   (i32.const 100)
 )
 (elem declare func $h)

 (func $entry (export "entry") (result i32)
   (block $handler (result (ref $ct1))
     (cont.new $ct0 (ref.func $f))
     (resume $ct0 (on $t switch) (on $t $handler))
     (return)
   )
   ;; we will not suspend
   (unreachable)
 )
)
(assert_return (invoke "entry" ) (i32.const 100))

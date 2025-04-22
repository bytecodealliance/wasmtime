;;! stack_switching = true

;; switch to continuation created by suspend
(module

 (type $ft0 (func (result i32)))
 (type $ct0 (cont $ft0))

 (type $ft1 (func (result i32)))
 (type $ct1 (cont $ft1))

 (type $ft2 (func (param i32 (ref $ct0)) (result i32)))
 (type $ct2 (cont $ft2))

 (tag $t_suspend (result i32 (ref $ct0)))
 (tag $t_switch (result i32))

 (global $c (mut (ref null $ct2)) (ref.null $ct2))


 (func $f (result i32)
   (suspend $t_suspend)
   (drop) ;; drops continuation created by switch without running to completion

   ;; We increment the switch payload and return it to our handler
   (i32.const 1)
   (i32.add)
 )
 (elem declare func $f)

 (func $g (result i32)
   (i32.const 100)
   (global.get $c)
   (switch $ct2 $t_switch)
   ;; we never switch back here
   (unreachable)
 )

 (elem declare func $g)

 (func $entry (export "entry") (result i32)
   (block $handler (result (ref $ct2))
     (cont.new $ct0 (ref.func $f))
     (resume $ct0 (on $t_suspend $handler))
     (unreachable)
   )
   (global.set $c)

   (cont.new $ct1 (ref.func $g))
   (resume $ct1 (on $t_switch switch))

 )
)
(assert_return (invoke "entry" ) (i32.const 101))

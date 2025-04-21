;;! stack_switching = true
;;! gc = true

;; resume continuation created by switch
(module

  (rec
    (type $ft0 (func (param i32 (ref null $ct0)) (result i32)))
    (type $ct0 (cont $ft0)))

  (type $ft1 (func (param i32) (result i32)))
  (type $ct1 (cont $ft1))

 (tag $t_switch (result i32))
 (tag $t_suspend (param i32) (result i32))

 (func $f0 (type $ft0)
   ;; Just a wrapper around $f1 to make sure that the involved continuation
   ;; chains consist of more than one element.

   (local.get 0)
   (local.get 1)
   (cont.new $ct0 (ref.func $f1))
   (resume $ct0)
 )
 (elem declare func $f0)

 (func $f1 (type $ft0)
   ;; add 1 to argument and pass to $g on switch
   (local.get 0)
   (i32.const 1)
   (i32.add)
   ;; prepare continuation
   (cont.new $ct0 (ref.func $g0))
   ;; switch to $g0
   (switch $ct0 $t_switch)
   ;; g1 resumed us, installed suspend handler for t_suspend)
   ;; drop null continuation and increment argument.
   (drop)
   (i32.const 1)
   (i32.add)
   (suspend $t_suspend)

   ;; add 1 to tag return value
   (i32.const 1)
   (i32.add)
 )
 (elem declare func $f1)

 (func $g0 (type $ft0)
   ;; Just a wrapper around $g1 to make sure that the involved continuation
   ;; chains consist of more than one element.

   (local.get 0)
   (local.get 1)
   (cont.new $ct0 (ref.func $g1))
   (resume $ct0)
 )
 (elem declare func $g0)

 (func $g1 (type $ft0)
  (local $c (ref $ct1))

  (block $handler (result i32 (ref $ct1))
    ;; add 1 to argument received from f1 on switch
    (local.get 0)
    (i32.const 1)
    (i32.add)
    (ref.null $ct0) ;; passed as payload
    (local.get 1) ;; resumed
    (resume $ct0 (on $t_suspend $handler))
    (unreachable) ;; f1 will suspend after the switch
  )
  ;; stash continuation created by suspend in $f1 aside
  (local.set $c)
  ;; increment value received from suspend in $f1
  (i32.const 1)
  (i32.add)
  ;; ... and pass back to $f1
  (local.get $c)
  (resume $ct1)
  ;; increment $f1's return value
  (i32.const 1)
  (i32.add)
 )
 (elem declare func $g1)

 (func $entry (export "entry") (result i32)
   (i32.const 100)
   (ref.null $ct0)
   (cont.new $ct0 (ref.func $f0))
   (resume $ct0 (on $t_switch switch))
 )
)
(assert_return (invoke "entry" ) (i32.const 106))

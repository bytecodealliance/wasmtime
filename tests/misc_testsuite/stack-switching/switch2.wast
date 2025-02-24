;;! stack_switching = true
;;! gc = true

;; switch to continuation created by switch
(module

  (rec
    (type $ft (func (param i32 (ref null $ct)) (result i32)))
    (type $ct (cont $ft)))

 (tag $t (result i32))

 (func $f0 (type $ft)
   ;; Just a wrapper around $f1 to make sure that the involved continuation
   ;; chains consist of more than one element.

   (local.get 0)
   (local.get 1)
   (cont.new $ct (ref.func $f1))
   (resume $ct)
 )
 (elem declare func $f0)

 (func $f1 (type $ft)
   ;; add 1 to argument and pass to $g0 on switch
   (local.get 0)
   (i32.const 1)
   (i32.add)
   ;; prepare continuation
   (cont.new $ct (ref.func $g0))
   (switch $ct $t)
   (drop) ;; we won't run $g to completion

   ;; add 1 to payload received from $g1
   (i32.const 1)
   (i32.add)
 )
 (elem declare func $f1)

 (func $g0 (type $ft)
   ;; Just a wrapper around $g1 to make sure that the involved continuation
   ;; chains consist of more than one element.

   (local.get 0)
   (local.get 1)
   (cont.new $ct (ref.func $g1))
   (resume $ct)
 )
 (elem declare func $g0)

 (func $g1 (type $ft)
  ;; add 1 to argument received from $f1
  (local.get 0)
  (i32.const 1)
  (i32.add)
  (local.get 1)
  (switch $ct $t)

  ;; $f never switches back to us
  (unreachable)
 )
 (elem declare func $g1)

 (func $entry (export "entry") (result i32)
   (i32.const 100)
   (ref.null $ct)
   (cont.new $ct (ref.func $f0))
   (resume $ct (on $t switch))
 )
)
(assert_return (invoke "entry" ) (i32.const 103))

;;! stack_switching = true
;;! gc = true

;; use cont.bind on continuation created by switch
(module

  (rec
    (type $ft0 (func (param     (ref null $ct1)) (result i32)))
    (type $ct0 (cont $ft0))
    (type $ft1 (func (param i32 (ref null $ct1)) (result i32)))
    (type $ct1 (cont $ft1)))

 (tag $t (result i32))

 (func $f (type $ft1)
   ;; add 1 to argument and pass to $g on switch
   (local.get 0)
   (i32.const 1)
   (i32.add)
   ;; prepare continuation
   (cont.new $ct1 (ref.func $g))
   (cont.bind $ct1 $ct0)
   (switch $ct0 $t)
   (drop) ;; we won't run $g to completion

   ;; add 1 to payload received from $g
   (i32.const 1)
   (i32.add)
 )
 (elem declare func $f)

 (func $g (type $ft1)
  ;; add 1 to argument received from $f
  (local.get 0)
  (i32.const 1)
  (i32.add)
  (local.get 1)
  (cont.bind $ct1 $ct0)
  (switch $ct0 $t)
  ;; $f never switches back to us
  (unreachable)
 )
 (elem declare func $g)

 (func $entry (export "entry") (result i32)
   (i32.const 100)
   (ref.null $ct1)
   (cont.new $ct1 (ref.func $f))
   (resume $ct1 (on $t switch))
 )
)
(assert_return (invoke "entry" ) (i32.const 103))

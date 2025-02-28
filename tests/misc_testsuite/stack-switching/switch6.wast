;;! stack_switching = true

;; suspend past a switch handler for same tag
(module

 (type $ft (func))
 (type $ct (cont $ft))

 (tag $t)

 (func $f
   (cont.new $ct (ref.func $g))
   (resume $ct (on $t switch))
   ;; $g will suspend and we will not come back here
   (unreachable)
 )
 (elem declare func $f)

 (func $g (type $ft)
   (suspend $t)
   ;; we will not come back here
   (unreachable)
 )
 (elem declare func $g)

 (func $entry (export "entry") (result i32)
   (block $handler (result (ref $ct))
     (cont.new $ct (ref.func $f))
     (resume $ct (on $t switch) (on $t $handler))
     ;; we will have a suspension
     (unreachable)
   )
   (drop)
   (i32.const 100)
 )
)
(assert_return (invoke "entry" ) (i32.const 100))

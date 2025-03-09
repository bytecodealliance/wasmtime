;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (tag $t)

  (func $f
    (suspend $t))
  (elem declare func $f)

  (func $dup (export "dup") (result i32)
    (block $on_t-1 (result (ref $ct))
      (block $on_t-2 (result (ref $ct))
        (resume $ct (on $t $on_t-1)
                    ;;(on $t $on_t-2)
                    (cont.new $ct (ref.func $f)))
        (return (i32.const 128))
      ) ;; on_t-2 [ (ref $ct) ]
      (drop)
      (return (i32.const 256))
    ) ;; on_t-1 [ (ref $ct) ]
    (drop)
    (return (i32.const 512)))
)

(assert_return (invoke "dup") (i32.const 512))
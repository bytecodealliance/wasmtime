;;! exceptions = true
;;! gc = true

(module
  (table $t 10 exnref)
  (global $g exnref (ref.null exn))
  (elem (table $t) (i32.const 0) exnref (global.get $g))
)

;;! gc = yes
;;! exceptions = yes
;;! function-references = yes

(module $m
  (global (export "") exnref (ref.null exn))) 

(module
  (import "m" "" (global exnref))
  (table 1 exnref (global.get 0)))

;;! target = "x86_64"
;;! test = "compile"

(module
  (type $t
    (func
      (result
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 10
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 20
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 30
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 40
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 50
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 60
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 70
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 80
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 90
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 100
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 110
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 120
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 130
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 140
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 150
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 160
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 170
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 180
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 190
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 200
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 210
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 220
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 230
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 240
        i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 ;; 250
        i32 i32 i32 i32                         ;; 254
      )
    )
  )
  (export "f" (func $f))
  (func $f (type $t) (unreachable))
)
;; wasm[0]::function[0]::f:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       ud2

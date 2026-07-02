;;! target = "x86_64"
;;! test = "compile"

(module
  (global $g (mut i32) (i32.const 0))

  ;; All of these should result in the constant being folded into the `mov`
  ;; instruction as an immediate operand, not materialized into a register.
  (func $f0
    (global.set $g (i32.const 0))
  )
  (func $f1
    (global.set $g (i32.const 1))
  )
  (func $f2
    (global.set $g (i32.const -1))
  )
  (func $f3
    (global.set $g (i32.const -10))
  )
  (func $f4
    (global.set $g (i32.const 100000))
  )
  (func $f5
    (global.set $g (i32.const 0x8fff_ffff))
  )
)
;; wasm[0]::function[0]::f0:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::f1:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $1, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::f2:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0xffffffff, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::f3:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0xfffffff6, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]::f4:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0x186a0, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]::f5:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0x8fffffff, 0x30(%rdi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

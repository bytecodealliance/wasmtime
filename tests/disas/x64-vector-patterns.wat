;;! target = "x86_64"
;;! test = "compile"

(module
  (func $zero (result v128) v128.const i64x2 0 0)
  (func $ones (result v128) v128.const i64x2 -1 -1)
)
;; wasm[0]::function[0]::zero:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::ones:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movdqu  0x14(%rip), %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   31: addb    %al, (%rax)
;;   33: addb    %al, (%rax)
;;   35: addb    %al, (%rax)
;;   37: addb    %al, (%rax)
;;   39: addb    %al, (%rax)
;;   3b: addb    %al, (%rax)
;;   3d: addb    %al, (%rax)
;;   3f: addb    %bh, %bh

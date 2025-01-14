;;! target = "x86_64"
;;! test = "winch"

(module 
  (import "env" "memory" (memory 1 1 shared))
  (func (i64.atomic.store32 (i32.const 0) (i64.const 42))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $0x2a, %rax
;;       movl    $0, %ecx
;;       andl    $3, %r11d
;;       cmpl    $0, %r11d
;;       jne     0x5d
;;   46: movq    0x58(%r14), %r11
;;       movq    (%r11), %rdx
;;       addq    %rcx, %rdx
;;       movl    %eax, (%rdx)
;;       mfence
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2
;;   5d: ud2

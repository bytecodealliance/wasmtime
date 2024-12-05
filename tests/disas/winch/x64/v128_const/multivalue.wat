;;! target = "x86_64"
;;! test = "winch"

(module
  (func (result v128 v128)
    v128.const i64x2 0 0
    v128.const i64x2 0 0
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x68
;;   1c: movq    %rsi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movdqu  0x36(%rip), %xmm0
;;       subq    $0x10, %rsp
;;       movdqu  0x29(%rip), %xmm15
;;       movdqu  %xmm15, 0x30(%rsp)
;;       movq    0x18(%rsp), %rax
;;       movdqu  (%rsp), %xmm15
;;       addq    $0x10, %rsp
;;       movdqu  %xmm15, (%rax)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   68: ud2
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rax)
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: addb    %al, (%rax)
;;   7a: addb    %al, (%rax)
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)

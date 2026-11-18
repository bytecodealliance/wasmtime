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
;;       ja      0x73
;;   1c: movq    %rsi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movdqu  0x43(%rip), %xmm0
;;       subq    $0x10, %rsp
;;       movdqu  0x33(%rip), %xmm15
;;       movdqu  %xmm15, (%rsp)
;;       movq    0x18(%rsp), %rax
;;       movdqu  (%rsp), %xmm15
;;       addq    $0x10, %rsp
;;       movdqu  %xmm15, (%rax)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   73: ud2
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rax)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
;;   7f: addb    %al, (%rax)
;;   81: addb    %al, (%rax)
;;   83: addb    %al, (%rax)
;;   85: addb    %al, (%rax)
;;   87: addb    %al, (%rax)
;;   89: addb    %al, (%rax)
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)

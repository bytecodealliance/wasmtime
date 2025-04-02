;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (result v128)
        (i16x8.neg (v128.const i64x2 0xFFFFFFFFFFFFFFFF 42)
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x43
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       vpxor   %xmm15, %xmm15, %xmm15
;;       vpsubw  %xmm0, %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   43: ud2
;;   45: addb    %al, (%rax)
;;   47: addb    %al, (%rax)
;;   49: addb    %al, (%rax)
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %bh, %bh

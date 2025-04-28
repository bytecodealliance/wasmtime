;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f32x4.pmax (v128.const f32x4 3 2 1 0) (v128.const f32x4 0 1 2 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x50
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x29(%rip), %xmm0
;;       movdqu  0x31(%rip), %xmm1
;;       vmaxps  %xmm1, %xmm0, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   50: ud2
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: addb    %al, (%rax)
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
;;   60: addb    %al, (%rax)
;;   62: addb    %al, (%rax)
;;   64: addb    %al, (%rax)
;;   66: cmpb    $0, (%rdi)
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   72: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   79: addb    %al, 0x3f(%rax)

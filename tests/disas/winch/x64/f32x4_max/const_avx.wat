;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f32x4.max (v128.const f32x4 3 2 1 0) (v128.const f32x4 0 1 2 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x70
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x49(%rip), %xmm0
;;       movdqu  0x51(%rip), %xmm1
;;       vmaxps  %xmm0, %xmm1, %xmm15
;;       vmaxps  %xmm1, %xmm0, %xmm1
;;       vxorps  %xmm15, %xmm1, %xmm1
;;       vorps   %xmm15, %xmm1, %xmm0
;;       vsubps  %xmm1, %xmm0, %xmm1
;;       vcmpunordps %xmm0, %xmm0, %xmm0
;;       vpsrld  $0xa, %xmm0, %xmm0
;;       vandnps %xmm1, %xmm0, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   70: ud2
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: addb    %al, (%rax)
;;   7a: addb    %al, (%rax)
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: addb    %al, (%rax)
;;   82: addb    %al, (%rax)
;;   84: addb    %al, (%rax)
;;   86: cmpb    $0, (%rdi)
;;   89: addb    %al, (%rax)
;;   8b: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   92: addb    %al, (%rax)
;;   96: addb    %al, (%rax)
;;   99: addb    %al, 0x3f(%rax)

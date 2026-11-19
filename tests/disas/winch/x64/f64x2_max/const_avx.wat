;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f64x2.max (v128.const i64x2 1 0) (v128.const i64x2 0 1))
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
;;       vmaxpd  %xmm0, %xmm1, %xmm15
;;       vmaxpd  %xmm1, %xmm0, %xmm1
;;       vxorpd  %xmm15, %xmm1, %xmm1
;;       vorpd   %xmm15, %xmm1, %xmm0
;;       vsubpd  %xmm1, %xmm0, %xmm1
;;       vcmpunordpd %xmm0, %xmm0, %xmm0
;;       vpsrlq  $0xd, %xmm0, %xmm0
;;       vandnpd %xmm1, %xmm0, %xmm1
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
;;   86: addb    %al, (%rax)
;;   88: addl    %eax, (%rax)
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   90: addl    %eax, (%rax)
;;   92: addb    %al, (%rax)
;;   94: addb    %al, (%rax)
;;   96: addb    %al, (%rax)
;;   98: addb    %al, (%rax)
;;   9a: addb    %al, (%rax)
;;   9c: addb    %al, (%rax)
;;   9e: addb    %al, (%rax)

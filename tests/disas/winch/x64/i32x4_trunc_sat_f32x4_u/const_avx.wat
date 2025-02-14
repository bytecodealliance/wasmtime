;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.trunc_sat_f32x4_u (v128.const f32x4 1 2 3 4))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x76
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x4c(%rip), %xmm0
;;       vxorps  %xmm0, %xmm0, %xmm15
;;       vmaxps  %xmm15, %xmm0, %xmm0
;;       vpcmpeqd %xmm15, %xmm15, %xmm15
;;       vpsrld  $1, %xmm15, %xmm15
;;       vcvtdq2ps %xmm15, %xmm15
;;       vcvttps2dq %xmm0, %xmm1
;;       vsubps  %xmm15, %xmm0, %xmm0
;;       vcmpleps %xmm0, %xmm15, %xmm15
;;       vcvttps2dq %xmm0, %xmm0
;;       vpxor   %xmm0, %xmm15, %xmm15
;;       vpxor   %xmm0, %xmm0, %xmm0
;;       vpmaxsd %xmm0, %xmm15, %xmm0
;;       vpaddd  %xmm1, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   76: ud2
;;   78: addb    %al, (%rax)
;;   7a: addb    %al, (%rax)
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: addb    %al, (%rax)
;;   82: cmpb    $0, (%rdi)
;;   85: addb    %al, (%rax)
;;   87: addb    %al, (%rax)
;;   8a: addb    %al, (%rax)

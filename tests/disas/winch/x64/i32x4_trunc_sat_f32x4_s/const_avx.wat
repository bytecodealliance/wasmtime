;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.trunc_sat_f32x4_s (v128.const f32x4 1 2 3 4))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x62
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       vcmpeqps %xmm0, %xmm0, %xmm15
;;       vandps  %xmm15, %xmm0, %xmm0
;;       vpxor   %xmm0, %xmm15, %xmm15
;;       vcvttps2dq %xmm0, %xmm0
;;       vpand   %xmm15, %xmm0, %xmm15
;;       vpsrad  $0x1f, %xmm15, %xmm15
;;       vpxor   %xmm15, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   62: ud2
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addb    %al, (%rax)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rax)
;;   72: cmpb    $0, (%rdi)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rax)
;;   7a: addb    %al, (%rax)

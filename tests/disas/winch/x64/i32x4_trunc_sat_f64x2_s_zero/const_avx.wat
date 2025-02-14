;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.trunc_sat_f64x2_s_zero (v128.const f32x4 1 2 3 4))
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
;;       movdqu  0x2c(%rip), %xmm0
;;       vcmpeqpd %xmm0, %xmm0, %xmm15
;;       vandpd  0x2f(%rip), %xmm15, %xmm15
;;       vminpd  %xmm15, %xmm0, %xmm0
;;       vcvttpd2dq %xmm0, %xmm0
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
;;   62: cmpb    $0, (%rdi)
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rax)
;;   6a: addb    %al, (%rax)
;;   6e: addb    $0, (%rax)
;;   72: sarb    $0xff, %bh

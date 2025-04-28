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
;;       ja      0x5f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       vcmpeqps %xmm0, %xmm0, %xmm15
;;       vandps  %xmm0, %xmm15, %xmm0
;;       vpxor   %xmm0, %xmm15, %xmm15
;;       vcvttps2dq %xmm0, %xmm0
;;       vpand   %xmm0, %xmm15, %xmm15
;;       vpsrad  $0x1f, %xmm15, %xmm15
;;       vpxor   %xmm0, %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5f: ud2
;;   61: addb    %al, (%rax)
;;   63: addb    %al, (%rax)
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rax)
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
;;   71: addb    %al, 0x3f(%rax)
;;   77: addb    %al, (%rax)
;;   7a: addb    %al, (%rax)

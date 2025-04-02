;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ] 

(module
    (func (result v128)
        (f32x4.convert_i32x4_u (v128.const i32x4 0 1 2 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x60
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x3c(%rip), %xmm0
;;       vpslld  $0x10, %xmm0, %xmm15
;;       vpsrld  $0x10, %xmm15, %xmm15
;;       vpsubd  %xmm15, %xmm0, %xmm0
;;       vcvtdq2ps %xmm15, %xmm15
;;       vpsrld  $1, %xmm0, %xmm0
;;       vcvtdq2ps %xmm0, %xmm0
;;       vaddps  %xmm0, %xmm0, %xmm0
;;       vaddps  %xmm0, %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   60: ud2
;;   62: addb    %al, (%rax)
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addb    %al, (%rax)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rax)
;;   72: addb    %al, (%rax)
;;   74: addl    %eax, (%rax)
;;   76: addb    %al, (%rax)
;;   78: addb    (%rax), %al
;;   7a: addb    %al, (%rax)
;;   7c: addl    (%rax), %eax
;;   7e: addb    %al, (%rax)

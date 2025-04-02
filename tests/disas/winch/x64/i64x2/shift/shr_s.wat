;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i64x2.shr_s (v128.const i64x2 1 2) (i32.const 3))
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
;;       movl    $3, %eax
;;       movdqu  0x37(%rip), %xmm0
;;       andl    $0x3f, %eax
;;       vmovd   %eax, %xmm15
;;       vmovdqu 0x38(%rip), %xmm1
;;       vpsrlq  %xmm15, %xmm1, %xmm1
;;       vpsrlq  %xmm15, %xmm0, %xmm0
;;       vpxor   %xmm1, %xmm0, %xmm0
;;       vpsubq  %xmm1, %xmm0, %xmm0
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
;;   70: addl    %eax, (%rax)
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: addb    (%rax), %al
;;   7a: addb    %al, (%rax)
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: addb    %al, (%rax)
;;   82: addb    %al, (%rax)
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)

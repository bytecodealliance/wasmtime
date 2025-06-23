;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result i32)
        (i8x16.all_true (v128.const i8x16 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x58
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x29(%rip), %xmm0
;;       vpxor   %xmm15, %xmm15, %xmm15
;;       vpcmpeqb %xmm15, %xmm0, %xmm0
;;       vptest  %xmm0, %xmm0
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   58: ud2
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
;;   60: addb    %al, (%rcx)
;;   62: addb    (%rbx), %al
;;   64: addb    $5, %al

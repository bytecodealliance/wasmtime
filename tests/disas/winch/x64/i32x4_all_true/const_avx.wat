;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result i32)
        (i32x4.all_true (v128.const i32x4 0 1 2 3))
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
;;       vpcmpeqd %xmm15, %xmm0, %xmm0
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
;;   60: addb    %al, (%rax)
;;   62: addb    %al, (%rax)
;;   64: addl    %eax, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addb    (%rax), %al
;;   6a: addb    %al, (%rax)
;;   6c: addl    (%rax), %eax
;;   6e: addb    %al, (%rax)

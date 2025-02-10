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
;;       ja      0x51
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       vpxor   %xmm15, %xmm15, %xmm15
;;       vpcmpeqb %xmm0, %xmm15, %xmm0
;;       vptest  %xmm0, %xmm0
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   51: ud2
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rax)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %al, (%rax)
;;   61: addl    %eax, (%rdx)
;;   63: addl    0x9080706(, %rax), %eax
;;   6a: orb     (%rbx), %cl
;;   6c: orb     $0xd, %al

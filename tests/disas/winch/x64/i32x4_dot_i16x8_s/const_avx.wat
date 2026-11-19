;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.dot_i16x8_s (v128.const i32x4 0 1 2 3) (v128.const i32x4 3 2 1 0))
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
;;       movdqu  0x29(%rip), %xmm0
;;       movdqu  0x31(%rip), %xmm1
;;       vpmaddwd %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
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
;;   60: addl    (%rax), %eax
;;   62: addb    %al, (%rax)
;;   64: addb    (%rax), %al
;;   66: addb    %al, (%rax)
;;   68: addl    %eax, (%rax)
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

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
;;       ja      0x4a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       movdqu  0x24(%rip), %xmm1
;;       vpmaddwd %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4a: ud2
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: addl    (%rax), %eax
;;   52: addb    %al, (%rax)
;;   54: addb    (%rax), %al
;;   56: addb    %al, (%rax)
;;   58: addl    %eax, (%rax)
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

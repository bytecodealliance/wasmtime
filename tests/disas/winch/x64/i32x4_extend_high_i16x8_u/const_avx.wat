;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.extend_high_i16x8_u (v128.const i16x8 0 1 2 3 4 5 6 7))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x44
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       vpxor   %xmm15, %xmm15, %xmm15
;;       vpunpckhwd %xmm15, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   44: ud2
;;   46: addb    %al, (%rax)
;;   48: addb    %al, (%rax)
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: addb    %al, (%rax)
;;   52: addl    %eax, (%rax)
;;   54: addb    (%rax), %al
;;   56: addl    (%rax), %eax
;;   58: addb    $0, %al
;;   5a: addl    $0x7000600, %eax

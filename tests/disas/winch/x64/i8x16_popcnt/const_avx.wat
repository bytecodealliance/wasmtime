;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i8x16.popcnt (v128.const i8x16 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       vpand   0x41(%rip), %xmm0, %xmm15
;;       vpsrlw  $4, %xmm0, %xmm0
;;       vpand   0x34(%rip), %xmm0, %xmm0
;;       movdqu  0x3c(%rip), %xmm1
;;       vpshufb %xmm0, %xmm1, %xmm0
;;       vpshufb %xmm15, %xmm1, %xmm15
;;       vpaddb  %xmm15, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6c: ud2
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rcx)
;;   72: addb    (%rbx), %al
;;   74: addb    $5, %al

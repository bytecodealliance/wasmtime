;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result i32)
        (i8x16.bitmask (v128.const i8x16 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0xc(%rip), %xmm0
;;       vpmovmskb %xmm0, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3e: ud2
;;   40: addb    %al, (%rcx)
;;   42: addb    (%rbx), %al
;;   44: addb    $5, %al

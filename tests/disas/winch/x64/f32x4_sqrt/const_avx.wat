;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f32x4.sqrt (v128.const f32x4 0 1 2 3))
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
;;       vsqrtps %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3e: ud2
;;   40: addb    %al, (%rax)
;;   42: addb    %al, (%rax)
;;   44: addb    %al, (%rax)
;;   46: cmpb    $0, (%rdi)
;;   49: addb    %al, (%rax)
;;   4b: addb    %al, (%rax)

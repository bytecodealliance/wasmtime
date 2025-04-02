;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.extend_high_i16x8_s (v128.const i16x8 0 1 2 3 4 5 6 7))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x45
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       vpalignr $8, %xmm0, %xmm0, %xmm0
;;       vpmovsxwd %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   45: ud2
;;   47: addb    %al, (%rax)
;;   49: addb    %al, (%rax)
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rax)
;;   51: addb    %al, (%rcx)
;;   53: addb    %al, (%rdx)
;;   55: addb    %al, (%rbx)
;;   57: addb    %al, (%rax, %rax)
;;   5a: addl    $0x7000600, %eax

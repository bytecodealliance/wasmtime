;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i64x2.extend_low_i32x4_s (v128.const i32x4 0 1 2 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       vpmovsxdq %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3f: ud2
;;   41: addb    %al, (%rax)
;;   43: addb    %al, (%rax)
;;   45: addb    %al, (%rax)
;;   47: addb    %al, (%rax)
;;   49: addb    %al, (%rax)
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rax)
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rcx)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rdx)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rbx)
;;   5d: addb    %al, (%rax)

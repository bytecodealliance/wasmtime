;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx2" ]

(module
    (func (result v128)
        (f32x4.splat (f32.const 0))
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
;;       movss   0x14(%rip), %xmm0
;;       vpbroadcastd %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3f: ud2
;;   41: addb    %al, (%rax)
;;   43: addb    %al, (%rax)
;;   45: addb    %al, (%rax)
;;   47: addb    %al, (%rax)
;;   49: addb    %al, (%rax)

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.abs)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4d
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x1d(%rip), %xmm0
;;       movabsq $0x7fffffffffffffff, %r11
;;       movq    %r11, %xmm15
;;       andpd   %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4d: ud2
;;   4f: addb    %bl, (%rdi)
;;   51: testl   %ebp, %ebx
;;   53: pushq   %rcx

;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_sse41"]

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.floor)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3f
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x15(%rip), %xmm0
;;       roundsd $1, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3f: ud2
;;   41: addb    %al, (%rax)
;;   43: addb    %al, (%rax)
;;   45: addb    %al, (%rax)
;;   47: addb    %bl, (%rdi)
;;   49: testl   %ebp, %ebx
;;   4b: pushq   %rcx

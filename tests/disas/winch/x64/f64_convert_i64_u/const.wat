;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (i64.const 1)
        (f64.convert_i64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $1, %rcx
;;       cmpq    $0, %rcx
;;       jl      0x4a
;;   40: cvtsi2sdq %rcx, %xmm0
;;       jmp     0x64
;;   4a: movq    %rcx, %r11
;;       shrq    $1, %r11
;;       movq    %rcx, %rax
;;       andq    $1, %rax
;;       orq     %r11, %rax
;;       cvtsi2sdq %rax, %xmm0
;;       addsd   %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6d: ud2

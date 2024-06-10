;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (local i64)  

        (local.get 0)
        (f64.convert_i64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6c
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movq    (%rsp), %rcx
;;       cmpq    $0, %rcx
;;       jl      0x4c
;;   42: cvtsi2sdq %rcx, %xmm0
;;       jmp     0x66
;;   4c: movq    %rcx, %r11
;;       shrq    $1, %r11
;;       movq    %rcx, %rax
;;       andq    $1, %rax
;;       orq     %r11, %rax
;;       cvtsi2sdq %rax, %xmm0
;;       addsd   %xmm0, %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   6c: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local i32)  

        (local.get 0)
        (f32.convert_i32_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x70
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    0xc(%rsp), %ecx
;;       movl    %ecx, %ecx
;;       cmpq    $0, %rcx
;;       jl      0x50
;;   46: cvtsi2ssq %rcx, %xmm0
;;       jmp     0x6a
;;   50: movq    %rcx, %r11
;;       shrq    $1, %r11
;;       movq    %rcx, %rax
;;       andq    $1, %rax
;;       orq     %r11, %rax
;;       cvtsi2ssq %rax, %xmm0
;;       addss   %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   70: ud2

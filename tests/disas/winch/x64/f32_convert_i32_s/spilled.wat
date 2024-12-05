;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        i32.const 1
        f32.convert_i32_s
        block
        end
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x14, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       cvtsi2ssl %eax, %xmm0
;;       subq    $4, %rsp
;;       movss   %xmm0, (%rsp)
;;       movss   (%rsp), %xmm0
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4d: ud2

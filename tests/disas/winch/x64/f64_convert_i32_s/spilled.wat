;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_s
        block
        end
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4c
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       cvtsi2sdl %eax, %xmm0
;;       subq    $8, %rsp
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       addq    $8, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4c: ud2

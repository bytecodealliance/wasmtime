;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.abs)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4d
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   0xc(%rsp), %xmm0
;;       movl    $0x7fffffff, %r11d
;;       movd    %r11d, %xmm15
;;       andps   %xmm15, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   4d: ud2

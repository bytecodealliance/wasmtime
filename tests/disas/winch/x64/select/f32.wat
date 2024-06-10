;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-f32") (param f32 f32 i32) (result f32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5f
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   %xmm1, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       movss   8(%rsp), %xmm0
;;       movss   0xc(%rsp), %xmm1
;;       cmpl    $0, %eax
;;       je      0x59
;;   55: movsd   %xmm1, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   5f: ud2

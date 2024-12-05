;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x60
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movsd   %xmm0, 0x18(%rsp)
;;       movsd   %xmm1, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movsd   0x10(%rsp), %xmm0
;;       movsd   0x18(%rsp), %xmm1
;;       cmpl    $0, %eax
;;       je      0x5a
;;   56: movsd   %xmm1, %xmm0
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   60: ud2

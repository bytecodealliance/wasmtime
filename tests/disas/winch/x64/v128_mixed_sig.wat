;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "x")
    (param f32 f32 f32 f32 f32 f32 f32 f32 f32) (param $last v128)
    (result v128)
    local.get $last
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x67
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movss   %xmm0, 0x1c(%rsp)
;;       movss   %xmm1, 0x18(%rsp)
;;       movss   %xmm2, 0x14(%rsp)
;;       movss   %xmm3, 0x10(%rsp)
;;       movss   %xmm4, 0xc(%rsp)
;;       movss   %xmm5, 8(%rsp)
;;       movss   %xmm6, 4(%rsp)
;;       movss   %xmm7, (%rsp)
;;       movdqu  0x20(%rbp), %xmm0
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   67: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (param f32) (result f32)
    local.get 0
    block
    end
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x1c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x52
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movss   %xmm0, 4(%rsp)
;;       movss   4(%rsp), %xmm15
;;       subq    $4, %rsp
;;       movss   %xmm15, (%rsp)
;;       movss   (%rsp), %xmm0
;;       addq    $4, %rsp
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   52: ud2

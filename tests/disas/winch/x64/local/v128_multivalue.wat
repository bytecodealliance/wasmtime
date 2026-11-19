;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (param v128) (result i64 v128 i64)
    i64.const 0
    local.get 0
    i64.const 0
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x48, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x9d
;;   1c: movq    %rsi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rsi, 0x28(%rsp)
;;       movq    %rdx, 0x20(%rsp)
;;       movdqu  %xmm0, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movl    $0, %eax
;;       movdqu  0x10(%rsp), %xmm15
;;       subq    $0x10, %rsp
;;       movdqu  %xmm15, (%rsp)
;;       subq    $8, %rsp
;;       movq    8(%rsp), %r11
;;       movq    %r11, (%rsp)
;;       movq    0x10(%rsp), %r11
;;       movq    %r11, 8(%rsp)
;;       movq    $0, 0x10(%rsp)
;;       movq    0x20(%rsp), %rcx
;;       movdqu  (%rsp), %xmm15
;;       addq    $0x10, %rsp
;;       movdqu  %xmm15, (%rcx)
;;       popq    %r11
;;       movq    %r11, 0x10(%rcx)
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   9d: ud2

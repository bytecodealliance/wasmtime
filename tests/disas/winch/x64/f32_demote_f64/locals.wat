;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local f64)  

        (local.get 0)
        (f32.demote_f64)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x43
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       cvtsd2ss %xmm0, %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   43: ud2

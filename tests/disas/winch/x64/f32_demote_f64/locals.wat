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
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x46
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movsd   8(%rsp), %xmm0
;;       cvtsd2ss %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   46: ud2

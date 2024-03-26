;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.reinterpret_f64)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3e
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0xd(%rip), %xmm0
;;       movq    %xmm0, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3e: ud2
;;   40: addb    %al, (%rax)
;;   42: addb    %al, (%rax)
;;   44: addb    %al, (%rax)

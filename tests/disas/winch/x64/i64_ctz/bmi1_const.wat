;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_bmi1"]

(module
    (func (result i64)
        (i64.const 1)
        (i64.ctz)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $1, %rax
;;       tzcntq  %rax, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2

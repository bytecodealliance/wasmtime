;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 1)
        (i64.clz)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x50
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $1, %rax
;;       bsrq    %rax, %rax
;;       movl    $0, %r11d
;;       setne   %r11b
;;       negq    %rax
;;       addq    $0x40, %rax
;;       subq    %r11, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   50: ud2

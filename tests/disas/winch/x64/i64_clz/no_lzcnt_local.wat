;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local $foo i64)

        (i64.const 2)
        (local.set $foo)

        (local.get $foo)
        (i64.clz)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x65
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movq    $2, %rax
;;       movq    %rax, 8(%rsp)
;;       movq    8(%rsp), %rax
;;       bsrq    %rax, %rax
;;       movl    $0, %r11d
;;       setne   %r11b
;;       negq    %rax
;;       addq    $0x40, %rax
;;       subq    %r11, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   65: ud2

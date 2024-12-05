;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)

        (i32.const 2)
        (local.set $foo)

        (local.get $foo)
        (i32.eqz)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x55
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    $2, %eax
;;       movl    %eax, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       cmpl    $0, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   55: ud2

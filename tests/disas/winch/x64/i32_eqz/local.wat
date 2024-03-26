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
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x53
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    $2, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       cmpl    $0, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   53: ud2

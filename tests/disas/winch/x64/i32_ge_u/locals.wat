;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)
        (local $bar i32)

        (i32.const 2)
        (local.set $foo)
        (i32.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.ge_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x63
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    $2, %eax
;;       movl    %eax, 0xc(%rsp)
;;       movl    $3, %eax
;;       movl    %eax, 8(%rsp)
;;       movl    8(%rsp), %eax
;;       movl    0xc(%rsp), %ecx
;;       cmpl    %eax, %ecx
;;       movl    $0, %ecx
;;       setae   %cl
;;       movl    %ecx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   63: ud2

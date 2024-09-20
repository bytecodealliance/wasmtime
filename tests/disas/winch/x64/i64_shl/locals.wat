;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 1)
        (local.set $foo)

        (i64.const 2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i64.shl)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x61
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       xorq    %r11, %r11
;;       movq    %r11, 8(%rsp)
;;       movq    %r11, (%rsp)
;;       movq    $1, %rax
;;       movq    %rax, 8(%rsp)
;;       movq    $2, %rax
;;       movq    %rax, (%rsp)
;;       movq    (%rsp), %rcx
;;       movq    8(%rsp), %rax
;;       shlq    %cl, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   61: ud2

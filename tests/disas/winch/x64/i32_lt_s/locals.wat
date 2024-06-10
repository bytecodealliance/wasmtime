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
        (i32.lt_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5f
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    $2, %eax
;;       movl    %eax, 4(%rsp)
;;       movl    $3, %eax
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %eax
;;       movl    4(%rsp), %ecx
;;       cmpl    %eax, %ecx
;;       movl    $0, %ecx
;;       setl    %cl
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   5f: ud2

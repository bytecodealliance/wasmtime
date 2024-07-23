;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_lzcnt"]

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
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x51
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movq    $2, %rax
;;       movq    %rax, 8(%rsp)
;;       movq    8(%rsp), %rax
;;       lzcntq  %rax, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   51: ud2

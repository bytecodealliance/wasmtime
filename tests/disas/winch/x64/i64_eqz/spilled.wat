;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        i64.const 1
        i64.eqz
        block
        end
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x14, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x53
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $1, %rax
;;       cmpq    $0, %rax
;;       movl    $0, %eax
;;       sete    %al
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   53: ud2

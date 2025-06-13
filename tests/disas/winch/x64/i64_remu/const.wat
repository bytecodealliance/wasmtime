;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 7)
	(i64.const 5)
	(i64.rem_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $5, %ecx
;;       movl    $7, %eax
;;       xorq    %rdx, %rdx
;;       divq    %rcx
;;       movq    %rdx, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4b: ud2

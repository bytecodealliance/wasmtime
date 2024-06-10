;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 20)
	(i64.const 10)
	(i64.div_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4e
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $0xa, %rcx
;;       movq    $0x14, %rax
;;       cmpq    $0, %rcx
;;       je      0x50
;;   43: cqto
;;       idivq   %rcx
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4e: ud2
;;   50: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.div_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x52
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $18446744073709551615, %rcx
;;       movabsq $9223372036854775808, %rax
;;       cmpq    $0, %rcx
;;       je      0x54
;;   47: cqto
;;       idivq   %rcx
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   52: ud2
;;   54: ud2

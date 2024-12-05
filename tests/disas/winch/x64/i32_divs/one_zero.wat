;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 1)
	(i32.const 0)
	(i32.div_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x48
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %ecx
;;       movl    $1, %eax
;;       cmpl    $0, %ecx
;;       je      0x4a
;;   3f: cltd
;;       idivl   %ecx
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   48: ud2
;;   4a: ud2

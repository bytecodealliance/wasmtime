;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 0)
	(i32.const 0)
	(i32.rem_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x53
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %ecx
;;       movl    $0, %eax
;;       cltd
;;       cmpl    $-1, %ecx
;;       jne     0x49
;;   3f: movl    $0, %edx
;;       jmp     0x4b
;;   49: idivl   %ecx
;;       movl    %edx, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   53: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.rem_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x58
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    (%rsp), %ecx
;;       movl    4(%rsp), %eax
;;       cltd
;;       cmpl    $-1, %ecx
;;       jne     0x4e
;;   44: movl    $0, %edx
;;       jmp     0x50
;;   4e: idivl   %ecx
;;       movl    %edx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   58: ud2

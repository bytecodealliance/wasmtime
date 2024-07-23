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
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5a
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    8(%rsp), %ecx
;;       movl    0xc(%rsp), %eax
;;       cltd
;;       cmpl    $-1, %ecx
;;       jne     0x50
;;   46: movl    $0, %edx
;;       jmp     0x52
;;   50: idivl   %ecx
;;       movl    %edx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   5a: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const -1)
	(i32.rem_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x54
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0xffffffff, %ecx
;;       movl    $0x80000000, %eax
;;       cltd
;;       cmpl    $-1, %ecx
;;       jne     0x4a
;;   40: movl    $0, %edx
;;       jmp     0x4c
;;   4a: idivl   %ecx
;;       movl    %edx, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   54: ud2

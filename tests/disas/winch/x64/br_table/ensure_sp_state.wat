;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (result i32)
    block (result i32)
       i32.const 0
    end
    i32.const 0
    i32.const 0
    br_table 0
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x14, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6a
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    $0, %ecx
;;       movl    $0, %eax
;;       movl    $0, %edx
;;       cmpl    %ecx, %edx
;;       cmovbl  %edx, %ecx
;;       leaq    0xa(%rip), %r11
;;       movslq  (%r11, %rcx, 4), %rdx
;;       addq    %rdx, %r11
;;       jmpq    *%r11
;;   5c: addb    $0, %al
;;       addb    %al, (%rax)
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6a: ud2

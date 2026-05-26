;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 2)
  (func (export "f") (param i32) (result i32)
    local.get 0
    i32.atomic.load16_u offset=0x80000000
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x9c
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movl    $0x80000000, %r11d
;;       addl    %r11d, %eax
;;       andl    $1, %eax
;;       cmpl    $0, %eax
;;       jne     0x9e
;;   4f: movl    0xc(%rsp), %eax
;;       movq    0x40(%r14), %rcx
;;       movl    %eax, %edx
;;       movl    $0x80000002, %r11d
;;       addq    %r11, %rdx
;;       jb      0xa0
;;   68: cmpq    %rcx, %rdx
;;       ja      0xa2
;;   71: movq    0x38(%r14), %rbx
;;       movl    %eax, %eax
;;       addq    %rax, %rbx
;;       movl    $0x80000000, %r11d
;;       addq    %r11, %rbx
;;       movl    $0, %esi
;;       cmpq    %rcx, %rdx
;;       cmovaq  %rsi, %rbx
;;       movzwq  (%rbx), %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   9c: ud2
;;   9e: ud2
;;   a0: ud2
;;   a2: ud2

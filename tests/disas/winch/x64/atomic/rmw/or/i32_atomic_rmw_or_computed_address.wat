;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "_start") (param i32) (result i32)
    (i32.atomic.rmw.or
      (i32.add (i32.const 1) (local.get 0))
      (i32.xor (i32.const 1) (local.get 0)))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x2c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x95
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movl    $1, %ecx
;;       addl    %eax, %ecx
;;       movl    0xc(%rsp), %eax
;;       movl    $1, %edx
;;       xorl    %eax, %edx
;;       movl    %ecx, %eax
;;       andl    $3, %eax
;;       cmpl    $0, %eax
;;       jne     0x97
;;   5a: movq    0x38(%r14), %rax
;;       movl    %ecx, %ecx
;;       addq    %rcx, %rax
;;       pushq   %rax
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       movl    (%rdx), %eax
;;       movq    %rax, %r11
;;       orq     %rcx, %r11
;;       lock cmpxchgl %r11d, (%rdx)
;;       jne     0x7b
;;   8c: addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   95: ud2
;;   97: ud2

;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "_start") (param i32) (result i32)
    (i32.atomic.rmw.cmpxchg
      (i32.add (i32.const 1) (local.get 0))
      (i32.xor (i32.const 1) (local.get 0))
      (i32.xor (i32.const 2) (local.get 0)))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa5
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
;;       movl    0xc(%rsp), %eax
;;       movl    $2, %ebx
;;       xorl    %eax, %ebx
;;       movl    %ecx, %eax
;;       andl    $3, %eax
;;       cmpl    $0, %eax
;;       jne     0xa7
;;   65: movq    0x38(%r14), %rax
;;       movl    %ecx, %ecx
;;       addq    %rcx, %rax
;;       pushq   %rax
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ebx, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       popq    %rdx
;;       lock cmpxchgl %ecx, (%rdx)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   a5: ud2
;;   a7: ud2

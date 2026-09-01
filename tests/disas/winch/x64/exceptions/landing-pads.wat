;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

;; A call with tagged and default handlers records both landing pads.
(module
  (tag $e (param i32))
  (import "m" "may_throw" (func $may_throw (param i32)))

  (func (param i32) (result i32)
    (block $catch_all
      (block $tagged (result i32)
        (try_table (catch $e $tagged) (catch_all $catch_all)
          (call $may_throw (local.get 0)))
        (return (i32.const 0)))
      (return))
    (i32.const -1)))
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xdf
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movq    0x48(%r14), %rcx
;;       movq    0x38(%r14), %rax
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %rcx, %rdi
;;       movq    %r14, %rsi
;;       movl    0xc(%rsp), %edx
;;       callq   *%rax
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ├─╼ exception handler: tag=0, context at [SP+0x28], handler=0x8b
;;       ╰─╼ exception handler: default handler, context at [SP+0x28], handler=0x77
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0xc7
;;   77: movq    %rbp, %rsp
;;       subq    $0x20, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0xd1
;;   8b: movq    %rbp, %rsp
;;       subq    $0x20, %rsp
;;       movq    0x18(%rsp), %r14
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rax, %r11
;;       addq    $0x18, %r11
;;       cmpq    %rdx, %r11
;;       ja      0xe1
;;   b9: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    0x10(%rdx), %eax
;;       jmp     0xd6
;;   c7: movl    $0, %eax
;;       jmp     0xd6
;;   d1: movl    $0xffffffff, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   df: ud2
;;   e1: ud2

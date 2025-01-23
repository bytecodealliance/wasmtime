;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (memory.atomic.wait64 (i32.const 4) (i64.const 0) (i64.const -1))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x82
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $18446744073709551615, %rax
;;       movq    $0, %rcx
;;       movl    $4, %edx
;;       movl    $0, %ebx
;;       movl    %edx, %edx
;;       subq    $4, %rsp
;;       movl    %ebx, (%rsp)
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rax
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movl    0x1c(%rsp), %esi
;;       movq    0x14(%rsp), %rdx
;;       movq    0xc(%rsp), %rcx
;;       movq    4(%rsp), %r8
;;       callq   0x1a5
;;       addq    $4, %rsp
;;       addq    $0x1c, %rsp
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   82: ud2

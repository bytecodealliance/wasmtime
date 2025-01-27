;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (memory.atomic.wait32 offset=8
          (i32.const 4)
          (i32.const 0)
          (i64.const -1))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $18446744073709551615, %rax
;;       movl    $0, %ecx
;;       movl    $4, %edx
;;       addq    $8, %rdx
;;       pushq   %rdx
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       pushq   %rax
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    0x18(%rsp), %rdx
;;       movl    0x14(%rsp), %ecx
;;       movq    0xc(%rsp), %r8
;;       callq   0x19f
;;       addq    $0xc, %rsp
;;       addq    $0x14, %rsp
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7c: ud2

;;! target = "x86_64"
;;! test = "winch"
(module
  (memory 1)
  (func (export "i64_load8_s") (param $i i64) (result i64)
   (i64.store8 (i32.const 8) (local.get $i))
   (i64.load8_s (i32.const 8))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5b
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %rax
;;       movl    $8, %ecx
;;       movq    0x50(%r14), %rdx
;;       addq    %rcx, %rdx
;;       movb    %al, (%rdx)
;;       movl    $8, %eax
;;       movq    0x50(%r14), %rcx
;;       addq    %rax, %rcx
;;       movsbq  (%rcx), %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2

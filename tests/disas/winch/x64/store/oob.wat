;;! target = "x86_64"
;;! test = "winch"
;;! flags = " -O static-memory-maximum-size=0"
(module
  (memory 1)
  (func (export "foo") (param $i i32)
    i32.const 0
    (local.get $i)
    i32.store8 offset=4294967295
  )
)

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8c
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movl    $0, %ecx
;;       movq    0x48(%r14), %rdx
;;       movl    %ecx, %ebx
;;       movabsq $0x100000000, %r11
;;       addq    %r11, %rbx
;;       jb      0x8e
;;   56: cmpq    %rdx, %rbx
;;       ja      0x90
;;   5f: movq    0x40(%r14), %rsi
;;       addq    %rcx, %rsi
;;       movabsq $0xffffffff, %r11
;;       addq    %r11, %rsi
;;       movq    $0, %rdi
;;       cmpq    %rdx, %rbx
;;       cmovaq  %rdi, %rsi
;;       movb    %al, (%rsi)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   8c: ud2
;;   8e: ud2
;;   90: ud2

;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5b
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    $0, 8(%rsp)
;;       xorl    %r11d, %r11d
;;       movl    0xc(%rsp), %ecx
;;       movl    $0x11, %eax
;;       testl   %ecx, %ecx
;;       jne     0x55
;;   4c: movl    %eax, 0xc(%rsp)
;;       movl    $0xffffffff, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2

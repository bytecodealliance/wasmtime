;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "as-block-last-value") (param i32) (result i32)
    (block (result i32)
      (call $dummy) (call $dummy) (br_if 0 (i32.const 11) (local.get 0))
    )
  )
)
;; wasm[0]::function[0]::dummy:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x31
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   31: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa7
;;   5b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    0xc(%rsp), %ecx
;;       movl    $0xb, %eax
;;       testl   %ecx, %ecx
;;       jne     0xa1
;;   a1: addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   a7: ud2

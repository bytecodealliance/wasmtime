;;! target = "x86_64"
;;! test = "winch"
(module
  (table $t1 0 funcref)
  (func (export "size") (result i32)
    (table.size $t1))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x38
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %r11
;;       movl    0x60(%r11), %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   38: ud2

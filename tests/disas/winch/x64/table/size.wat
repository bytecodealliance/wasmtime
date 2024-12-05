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
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x39
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %r11
;;       movq    0x60(%r11), %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   39: ud2

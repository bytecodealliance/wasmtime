;;! target = "x86_64"
;;! test = "winch"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
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
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    %rdx, (%rsp)
;;       movq    (%rsp), %r11
;;       pushq   %r11
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    $0xa, %edx
;;       movq    (%rsp), %rcx
;;       callq   0x165
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2

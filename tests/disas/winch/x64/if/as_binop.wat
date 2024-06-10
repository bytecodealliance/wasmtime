;;! target = "x86_64"
;;! test = "winch"

(module
  (func $dummy)
  (func (export "as-binary-operand") (param i32 i32) (result i32)
    (i32.mul
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 3))
        (else (call $dummy) (i32.const -3))
      )
      (if (result i32) (local.get 1)
        (then (call $dummy) (i32.const 4))
        (else (call $dummy) (i32.const -5))
      )
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
;;       ja      0x121
;;   5b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    4(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xa1
;;   7f: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $3, %eax
;;       jmp     0xbe
;;   a1: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $0xfffffffd, %eax
;;       movl    (%rsp), %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       testl   %ecx, %ecx
;;       je      0xf2
;;   d0: subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       movl    $4, %eax
;;       jmp     0x10f
;;   f2: subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       movl    $0xfffffffb, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       imull   %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;  121: ud2

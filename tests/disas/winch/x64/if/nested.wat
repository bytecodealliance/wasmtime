;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
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
;;       ja      0x1b4
;;   5b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    4(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x119
;;   7f: movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xa2
;;   8a: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xb2
;;       jmp     0xca
;;   b2: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xf7
;;   d5: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $9, %eax
;;       jmp     0x1ae
;;   f7: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $0xa, %eax
;;       jmp     0x1ae
;;  119: movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x13c
;;  124: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x14c
;;       jmp     0x164
;;  14c: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x191
;;  16f: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $0xa, %eax
;;       jmp     0x1ae
;;  191: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $0xb, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;  1b4: ud2

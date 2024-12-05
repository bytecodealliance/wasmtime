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
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x32
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   32: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x17c
;;   5c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    0xc(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xfe
;;   81: movl    8(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x9d
;;   8d: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    8(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xae
;;       jmp     0xbe
;;   ae: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    8(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xe4
;;   ca: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    $9, %eax
;;       jmp     0x176
;;   e4: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    $0xa, %eax
;;       jmp     0x176
;;   fe: movl    8(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x11a
;;  10a: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    8(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x12b
;;       jmp     0x13b
;;  12b: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    8(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x161
;;  147: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    $0xa, %eax
;;       jmp     0x176
;;  161: movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       movl    $0xb, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  17c: ud2

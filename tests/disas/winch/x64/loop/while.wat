;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "while-") (param i64) (result i64)
    (local i64)
    (local.set 1 (i64.const 1))
    (block
      (loop
        (br_if 1 (i64.eqz (local.get 0)))
        (local.set 1 (i64.mul (local.get 0) (local.get 1)))
        (local.set 0 (i64.sub (local.get 0) (i64.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x8c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movq    %rdx, 8(%rsp)
;;   31: movq    $0, (%rsp)
;;   39: movq    $1, %rax
;;   40: movq    %rax, (%rsp)
;;   44: movq    8(%rsp), %rax
;;   49: cmpq    $0, %rax
;;   4d: movl    $0, %eax
;;   52: sete    %al
;;   56: testl   %eax, %eax
;;   58: jne     0x82
;;   5e: movq    (%rsp), %rax
;;   62: movq    8(%rsp), %rcx
;;   67: imulq   %rax, %rcx
;;   6b: movq    %rcx, (%rsp)
;;   6f: movq    8(%rsp), %rax
;;   74: subq    $1, %rax
;;   78: movq    %rax, 8(%rsp)
;;   7d: jmp     0x44
;;   82: movq    (%rsp), %rax
;;   86: addq    $0x20, %rsp
;;   8a: popq    %rbp
;;   8b: retq
;;   8c: ud2

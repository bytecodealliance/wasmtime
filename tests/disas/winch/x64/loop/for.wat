;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "for-") (param i64) (result i64)
    (local i64 i64)
    (local.set 1 (i64.const 1))
    (local.set 2 (i64.const 2))
    (block
      (loop
        (br_if 1 (i64.gt_u (local.get 2) (local.get 0)))
        (local.set 1 (i64.mul (local.get 1) (local.get 2)))
        (local.set 2 (i64.add (local.get 2) (i64.const 1)))
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
;;    b: addq    $0x28, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x9f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x28, %rsp
;;   22: movq    %rdi, 0x20(%rsp)
;;   27: movq    %rsi, 0x18(%rsp)
;;   2c: movq    %rdx, 0x10(%rsp)
;;   31: xorl    %r11d, %r11d
;;   34: movq    %r11, 8(%rsp)
;;   39: movq    %r11, (%rsp)
;;   3d: movq    $1, %rax
;;   44: movq    %rax, 8(%rsp)
;;   49: movq    $2, %rax
;;   50: movq    %rax, (%rsp)
;;   54: movq    0x10(%rsp), %rax
;;   59: movq    (%rsp), %rcx
;;   5d: cmpq    %rax, %rcx
;;   60: movl    $0, %ecx
;;   65: seta    %cl
;;   69: testl   %ecx, %ecx
;;   6b: jne     0x94
;;   71: movq    (%rsp), %rax
;;   75: movq    8(%rsp), %rcx
;;   7a: imulq   %rax, %rcx
;;   7e: movq    %rcx, 8(%rsp)
;;   83: movq    (%rsp), %rax
;;   87: addq    $1, %rax
;;   8b: movq    %rax, (%rsp)
;;   8f: jmp     0x54
;;   94: movq    8(%rsp), %rax
;;   99: addq    $0x28, %rsp
;;   9d: popq    %rbp
;;   9e: retq
;;   9f: ud2

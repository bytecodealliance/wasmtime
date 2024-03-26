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
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    $0, (%rsp)
;;   37: xorl    %r11d, %r11d
;;   3a: movl    4(%rsp), %ecx
;;   3e: movl    $0x11, %eax
;;   43: testl   %ecx, %ecx
;;   45: jne     0x54
;;   4b: movl    %eax, 4(%rsp)
;;   4f: movl    $0xffffffff, %eax
;;   54: addq    $0x18, %rsp
;;   58: popq    %rbp
;;   59: retq
;;   5a: ud2

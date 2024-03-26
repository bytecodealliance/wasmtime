;;! target = "x86_64"
;;! test = "winch"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-last") (result i32)
    (block (result i32)
      (call $f
        (i32.const 1) (i32.const 2) (br_if 0 (i32.const 14) (i32.const 1))
      )
    )
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x44
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movl    %edx, 0xc(%rsp)
;;   30: movl    %ecx, 8(%rsp)
;;   34: movl    %r8d, 4(%rsp)
;;   39: movl    $0xffffffff, %eax
;;   3e: addq    $0x20, %rsp
;;   42: popq    %rbp
;;   43: retq
;;   44: ud2
;;
;; wasm[0]::function[1]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: movq    8(%rdi), %r11
;;   58: movq    (%r11), %r11
;;   5b: addq    $0x20, %r11
;;   62: cmpq    %rsp, %r11
;;   65: ja      0xc5
;;   6b: movq    %rdi, %r14
;;   6e: subq    $0x10, %rsp
;;   72: movq    %rdi, 8(%rsp)
;;   77: movq    %rsi, (%rsp)
;;   7b: movl    $1, %ecx
;;   80: movl    $0xe, %eax
;;   85: testl   %ecx, %ecx
;;   87: jne     0xbf
;;   8d: subq    $4, %rsp
;;   91: movl    %eax, (%rsp)
;;   94: subq    $0xc, %rsp
;;   98: movq    %r14, %rdi
;;   9b: movq    %r14, %rsi
;;   9e: movl    $1, %edx
;;   a3: movl    $2, %ecx
;;   a8: movl    0xc(%rsp), %r8d
;;   ad: callq   0
;;   b2: addq    $0xc, %rsp
;;   b6: addq    $4, %rsp
;;   ba: movq    8(%rsp), %r14
;;   bf: addq    $0x10, %rsp
;;   c3: popq    %rbp
;;   c4: retq
;;   c5: ud2

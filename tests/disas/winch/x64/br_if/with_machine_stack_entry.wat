;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") 
    call 1
    call 1
    br_if 0
    drop
  )
  (func (;1;) (result i32)
    i32.const 1
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x75
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    %r14, %rdi
;;   2e: movq    %r14, %rsi
;;   31: callq   0x80
;;   36: movq    8(%rsp), %r14
;;   3b: subq    $4, %rsp
;;   3f: movl    %eax, (%rsp)
;;   42: subq    $0xc, %rsp
;;   46: movq    %r14, %rdi
;;   49: movq    %r14, %rsi
;;   4c: callq   0x80
;;   51: addq    $0xc, %rsp
;;   55: movq    0xc(%rsp), %r14
;;   5a: testl   %eax, %eax
;;   5c: je      0x6b
;;   62: addq    $4, %rsp
;;   66: jmp     0x6f
;;   6b: addq    $4, %rsp
;;   6f: addq    $0x10, %rsp
;;   73: popq    %rbp
;;   74: retq
;;   75: ud2
;;
;; wasm[0]::function[1]:
;;   80: pushq   %rbp
;;   81: movq    %rsp, %rbp
;;   84: movq    8(%rdi), %r11
;;   88: movq    (%r11), %r11
;;   8b: addq    $0x10, %r11
;;   92: cmpq    %rsp, %r11
;;   95: ja      0xb6
;;   9b: movq    %rdi, %r14
;;   9e: subq    $0x10, %rsp
;;   a2: movq    %rdi, 8(%rsp)
;;   a7: movq    %rsi, (%rsp)
;;   ab: movl    $1, %eax
;;   b0: addq    $0x10, %rsp
;;   b4: popq    %rbp
;;   b5: retq
;;   b6: ud2

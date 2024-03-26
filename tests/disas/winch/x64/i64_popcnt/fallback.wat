;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
      i64.const 15
      i64.popcnt
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x92
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movq    $0xf, %rax
;;   32: movq    %rax, %rcx
;;   35: shrq    $1, %rax
;;   39: movabsq $0x5555555555555555, %r11
;;   43: andq    %r11, %rax
;;   46: subq    %rax, %rcx
;;   49: movq    %rcx, %rax
;;   4c: movabsq $0x3333333333333333, %r11
;;   56: andq    %r11, %rax
;;   59: shrq    $2, %rcx
;;   5d: andq    %r11, %rcx
;;   60: addq    %rax, %rcx
;;   63: movq    %rcx, %rax
;;   66: shrq    $4, %rax
;;   6a: addq    %rcx, %rax
;;   6d: movabsq $0xf0f0f0f0f0f0f0f, %r11
;;   77: andq    %r11, %rax
;;   7a: movabsq $0x101010101010101, %r11
;;   84: imulq   %r11, %rax
;;   88: shrq    $0x38, %rax
;;   8c: addq    $0x10, %rsp
;;   90: popq    %rbp
;;   91: retq
;;   92: ud2

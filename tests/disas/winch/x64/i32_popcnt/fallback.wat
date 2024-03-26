;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
      i32.const 15
      i32.popcnt
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $0xf, %eax
;;   30: movl    %eax, %ecx
;;   32: shrl    $1, %eax
;;   35: andl    $0x55555555, %eax
;;   3b: subl    %eax, %ecx
;;   3d: movl    %ecx, %eax
;;   3f: movl    $0x33333333, %r11d
;;   45: andl    %r11d, %eax
;;   48: shrl    $2, %ecx
;;   4b: andl    %r11d, %ecx
;;   4e: addl    %eax, %ecx
;;   50: movl    %ecx, %eax
;;   52: shrl    $4, %eax
;;   55: addl    %ecx, %eax
;;   57: andl    $0xf0f0f0f, %eax
;;   5d: imull   $0x1010101, %eax, %eax
;;   63: shrl    $0x18, %eax
;;   66: addq    $0x10, %rsp
;;   6a: popq    %rbp
;;   6b: retq
;;   6c: ud2

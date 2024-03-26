;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (result i32)
    block (result i32)
       i32.const 0
    end
    i32.const 0
    i32.const 0
    br_table 0
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x14, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $0, %eax
;;   30: subq    $4, %rsp
;;   34: movl    %eax, (%rsp)
;;   37: movl    $0, %ecx
;;   3c: movl    $0, %eax
;;   41: movl    $0, %edx
;;   46: cmpl    %ecx, %edx
;;   48: cmovbl  %edx, %ecx
;;   4b: leaq    0xa(%rip), %r11
;;   52: movslq  (%r11, %rcx, 4), %rdx
;;   56: addq    %rdx, %r11
;;   59: jmpq    *%r11
;;   5c: addb    $0, %al
;;   5e: addb    %al, (%rax)
;;   60: addq    $4, %rsp
;;   64: addq    $0x10, %rsp
;;   68: popq    %rbp
;;   69: retq
;;   6a: ud2

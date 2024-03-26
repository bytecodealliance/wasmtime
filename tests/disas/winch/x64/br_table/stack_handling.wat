;;! target = "x86_64"
;;! test = "winch"
(module
  (func (;0;) (param i32)
    local.get 0
    block ;; label = @1
      i32.const 808727609
      br_table 0 (;@1;) 1 (;@0;) 0 (;@1;)
    end
    drop
  )
  (export "main" (func 0))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x1c, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x81
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %r11d
;;   35: subq    $4, %rsp
;;   39: movl    %r11d, (%rsp)
;;   3d: movl    $0x30343439, %eax
;;   42: movl    $2, %ecx
;;   47: cmpl    %eax, %ecx
;;   49: cmovbl  %ecx, %eax
;;   4c: leaq    0xa(%rip), %r11
;;   53: movslq  (%r11, %rax, 4), %rcx
;;   57: addq    %rcx, %r11
;;   5a: jmpq    *%r11
;;   5d: sbbb    (%rax), %al
;;   5f: addb    %al, (%rax)
;;   61: adcl    %eax, (%rax)
;;   63: addb    %al, (%rax)
;;   65: sbbb    (%rax), %al
;;   67: addb    %al, (%rax)
;;   69: jmp     0x77
;;   6e: addq    $4, %rsp
;;   72: jmp     0x7b
;;   77: addq    $4, %rsp
;;   7b: addq    $0x18, %rsp
;;   7f: popq    %rbp
;;   80: retq
;;   81: ud2

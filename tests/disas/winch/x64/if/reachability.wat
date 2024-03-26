;;! target = "x86_64"
;;! test = "winch"
(module
  (func (;0;) (param i32) (result i32)
    local.get 0
    local.get 0
    if (result i32)
      i32.const 1
        return
      else
        i32.const 2
      end
      i32.sub
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
;;   15: ja      0x6d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %eax
;;   34: movl    4(%rsp), %r11d
;;   39: subq    $4, %rsp
;;   3d: movl    %r11d, (%rsp)
;;   41: testl   %eax, %eax
;;   43: je      0x57
;;   49: movl    $1, %eax
;;   4e: addq    $4, %rsp
;;   52: jmp     0x67
;;   57: movl    $2, %eax
;;   5c: movl    (%rsp), %ecx
;;   5f: addq    $4, %rsp
;;   63: subl    %eax, %ecx
;;   65: movl    %ecx, %eax
;;   67: addq    $0x18, %rsp
;;   6b: popq    %rbp
;;   6c: retq
;;   6d: ud2

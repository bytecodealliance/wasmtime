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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x1c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6d
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       testl   %eax, %eax
;;       je      0x57
;;   49: movl    $1, %eax
;;       addq    $4, %rsp
;;       jmp     0x67
;;   57: movl    $2, %eax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       subl    %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   6d: ud2

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
;;       addq    $0x24, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6d
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movl    0xc(%rsp), %r11d
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
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   6d: ud2

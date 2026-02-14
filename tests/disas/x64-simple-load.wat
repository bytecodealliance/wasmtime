;;! target = "x86_64"
;;! test = "compile"
;;! objdump = '--traps --addrmap'

(module
  (memory 1)

  (func $load8 (param i32) (result i32 i32)
    (i32.load8_u (local.get 0))
    (i32.load8_u offset=4 (local.get 0))
  )
)
;; wasm[0]::function[0]::load8:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x38(%rdi), %rsi
;;       ╰─╼ addrmap: 0x21
;;       movl    %edx, %edi
;;       movzbq  (%rsi, %rdi), %rax
;;       ╰─╼ trap: MemoryOutOfBounds
;;       movzbq  4(%rsi, %rdi), %rcx
;;       ├─╼ addrmap: 0x26
;;       ╰─╼ trap: MemoryOutOfBounds
;;       movq    %rbp, %rsp
;;       ╰─╼ addrmap: 0x29
;;       popq    %rbp
;;       retq

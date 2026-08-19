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
;;       movq    0x38(%rdi), %r8
;;       ╰─╼ addrmap: 0x21
;;       movl    %edx, %r10d
;;       movzbq  (%r8, %r10), %r9
;;       ╰─╼ trap: Normal(MemoryOutOfBounds)
;;       movzbq  4(%r8, %r10), %r8
;;       ├─╼ addrmap: 0x26
;;       ╰─╼ trap: Normal(MemoryOutOfBounds)
;;       movzbl  %r9b, %eax
;;       ╰─╼ addrmap: 0x21
;;       movzbl  %r8b, %ecx
;;       ╰─╼ addrmap: 0x26
;;       movq    %rbp, %rsp
;;       ╰─╼ addrmap: 0x29
;;       popq    %rbp
;;       retq

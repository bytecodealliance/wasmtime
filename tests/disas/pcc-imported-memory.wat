;;! target = "x86_64"
;;! test = "compile"
;;! flags = [ "-Oopt-level=2", "-Cpcc=y" ]

(module
  (type (;0;) (func))
  (import "" "" (memory (;0;) 1))
  (func (;0;) (type 0)
    (local i32 i32)
    memory.size
    local.set 0
    block  ;; label = @1
      block  ;; label = @2
        memory.size
        i32.const 65536
        i32.mul
        i32.const 4
        local.get 0
        i32.add
        i32.le_u
        br_if 0 (;@2;)
        local.get 0
        i32.const 0
        i32.le_s
        br_if 0 (;@2;)
        local.get 0
        i32.load align=1
        local.set 1
        br 1 (;@1;)
      end
      i32.const 0
      local.set 1
    end
    local.get 1
    drop))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x58(%rdi), %rcx
;;       movq    8(%rcx), %rax
;;       shrq    $0x10, %rax
;;       movq    %rax, %rcx
;;       shll    $0x10, %ecx
;;       leal    4(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jbe     0x3b
;;   21: testl   %eax, %eax
;;       jle     0x3b
;;   29: movq    0x58(%rdi), %rsi
;;       movq    (%rsi), %rsi
;;       movl    %eax, %edi
;;       movl    (%rsi, %rdi), %r10d
;;       jmp     0x3e
;;   3b: xorl    %r10d, %r10d
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

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
;;       movq    0x30(%rdi), %rdx
;;       movq    8(%rdx), %rcx
;;       shrq    $0x10, %rcx
;;       movq    %rcx, %rdx
;;       shll    $0x10, %edx
;;       leal    4(%rcx), %r8d
;;       cmpl    %r8d, %edx
;;       jbe     0x3d
;;   23: testl   %ecx, %ecx
;;       jle     0x3d
;;   2b: movq    0x30(%rdi), %rdi
;;       movq    (%rdi), %rdi
;;       movl    %ecx, %eax
;;       movl    (%rdi, %rax), %r10d
;;       jmp     0x40
;;   3d: xorl    %r10d, %r10d
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq

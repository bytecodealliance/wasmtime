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
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    0x48(%rdi), %r10
;;    8: movq    8(%r10), %rax
;;    c: movl    $0x10000, %r10d
;;   12: xorq    %rdx, %rdx
;;   15: divq    %r10
;;   18: movq    %rax, %r9
;;   1b: shll    $0x10, %r9d
;;   1f: leal    4(%rax), %r10d
;;   23: cmpl    %r10d, %r9d
;;   26: jbe     0x45
;;   2c: testl   %eax, %eax
;;   2e: jle     0x45
;;   34: movq    0x48(%rdi), %rcx
;;   38: movq    (%rcx), %rcx
;;   3b: movl    %eax, %edx
;;   3d: movl    (%rcx, %rdx), %edi
;;   40: jmp     0x47
;;   45: xorl    %edi, %edi
;;   47: movq    %rbp, %rsp
;;   4a: popq    %rbp
;;   4b: retq

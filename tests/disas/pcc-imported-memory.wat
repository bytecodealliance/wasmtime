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

;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movq    72(%rdi), %r10
;;   movq    8(%r10), %rax
;;   movl    $65536, %r10d
;;   xorq    %rdx, %rdx, %rdx
;;   div     %rax, %rdx, %r10, %rax, %rdx ; trap=int_divz
;;   movq    %rax, %r9
;;   shll    $16, %r9d, %r9d
;;   lea     4(%rax), %r10d
;;   cmpl    %r10d, %r9d
;;   jbe     label1; j label2
;; block1:
;;   jmp     label5
;; block2:
;;   testl   %eax, %eax
;;   jle     label3; j label4
;; block3:
;;   jmp     label5
;; block4:
;;   movq    72(%rdi), %rcx
;;   movq    0(%rcx), %rcx
;;   movl    %eax, %edx
;;   movl    0(%rcx,%rdx,1), %edi
;;   jmp     label6
;; block5:
;;   xorl    %edi, %edi, %edi
;;   jmp     label6
;; block6:
;;   jmp     label7
;; block7:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret

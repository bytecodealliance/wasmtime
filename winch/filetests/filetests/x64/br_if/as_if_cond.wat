;;! target = "x86_64"
(module
  (func (export "as-if-cond") (param i32) (result i32)
    (block (result i32)
      (if (result i32)
        (br_if 0 (i32.const 1) (local.get 0))
        (then (i32.const 2))
        (else (i32.const 3))
      )
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   15:	 48c7c001000000       	mov	rax, 1
;;   1c:	 85c9                 	test	ecx, ecx
;;   1e:	 0f851b000000         	jne	0x3f
;;   24:	 85c0                 	test	eax, eax
;;   26:	 0f840c000000         	je	0x38
;;   2c:	 48c7c002000000       	mov	rax, 2
;;   33:	 e907000000           	jmp	0x3f
;;   38:	 48c7c003000000       	mov	rax, 3
;;   3f:	 4883c410             	add	rsp, 0x10
;;   43:	 5d                   	pop	rbp
;;   44:	 c3                   	ret	

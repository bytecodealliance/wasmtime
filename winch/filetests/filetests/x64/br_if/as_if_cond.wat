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
;;   15:	 b801000000           	mov	eax, 1
;;   1a:	 85c9                 	test	ecx, ecx
;;   1c:	 0f8517000000         	jne	0x39
;;   22:	 85c0                 	test	eax, eax
;;   24:	 0f840a000000         	je	0x34
;;   2a:	 b802000000           	mov	eax, 2
;;   2f:	 e905000000           	jmp	0x39
;;   34:	 b803000000           	mov	eax, 3
;;   39:	 4883c410             	add	rsp, 0x10
;;   3d:	 5d                   	pop	rbp
;;   3e:	 c3                   	ret	

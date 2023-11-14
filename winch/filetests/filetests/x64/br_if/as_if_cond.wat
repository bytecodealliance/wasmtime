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
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   14:	 b801000000           	mov	eax, 1
;;   19:	 85c9                 	test	ecx, ecx
;;   1b:	 0f8517000000         	jne	0x38
;;   21:	 85c0                 	test	eax, eax
;;   23:	 0f840a000000         	je	0x33
;;   29:	 b802000000           	mov	eax, 2
;;   2e:	 e905000000           	jmp	0x38
;;   33:	 b803000000           	mov	eax, 3
;;   38:	 4883c410             	add	rsp, 0x10
;;   3c:	 5d                   	pop	rbp
;;   3d:	 c3                   	ret	

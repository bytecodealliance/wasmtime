;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-if-then") (param i32 i32)
    (block
      (if (local.get 0) (then (br_if 1 (local.get 1))) (else (call $dummy)))
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f8411000000         	je	0x31
;;   20:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f850a000000         	jne	0x36
;;      	 e905000000           	jmp	0x36
;;   31:	 e800000000           	call	0x36
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	

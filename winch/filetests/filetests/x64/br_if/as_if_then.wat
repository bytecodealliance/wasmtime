;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-if-then") (param i32 i32)
    (block
      (if (local.get 0) (then (br_if 1 (local.get 1))) (else (call $dummy)))
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f8411000000         	je	0x31
;;   20:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   24:	 85c9                 	test	ecx, ecx
;;   26:	 0f850a000000         	jne	0x36
;;   2c:	 e905000000           	jmp	0x36
;;   31:	 e800000000           	call	0x36
;;   36:	 4883c410             	add	rsp, 0x10
;;   3a:	 5d                   	pop	rbp
;;   3b:	 c3                   	ret	

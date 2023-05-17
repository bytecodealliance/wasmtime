;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   18:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   1c:	 0fafc8               	imul	ecx, eax
;;   1f:	 4889c8               	mov	rax, rcx
;;   22:	 4883c410             	add	rsp, 0x10
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	

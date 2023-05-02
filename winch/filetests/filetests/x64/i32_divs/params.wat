;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   18:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   1c:	 83f900               	cmp	ecx, 0
;;   1f:	 0f8409000000         	je	0x2e
;;   25:	 99                   	cdq	
;;   26:	 f7f9                 	idiv	ecx
;;   28:	 4883c410             	add	rsp, 0x10
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	
;;   2e:	 0f0b                 	ud2	

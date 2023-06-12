;;! target = "x86_64"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 83f800               	cmp	eax, 0
;;   18:	 b800000000           	mov	eax, 0
;;   1d:	 400f94c0             	sete	al
;;   21:	 4883c410             	add	rsp, 0x10
;;   25:	 5d                   	pop	rbp
;;   26:	 c3                   	ret	

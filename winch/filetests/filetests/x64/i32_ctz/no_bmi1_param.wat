;;! target = "x86_64"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.ctz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 0fbcc0               	bsf	eax, eax
;;   17:	 41bb00000000         	mov	r11d, 0
;;   1d:	 410f94c3             	sete	r11b
;;   21:	 41c1e305             	shl	r11d, 5
;;   25:	 4401d8               	add	eax, r11d
;;   28:	 4883c410             	add	rsp, 0x10
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	

;;! target = "x86_64"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.clz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 0fbdc0               	bsr	eax, eax
;;   17:	 41bb00000000         	mov	r11d, 0
;;   1d:	 410f95c3             	setne	r11b
;;   21:	 f7d8                 	neg	eax
;;   23:	 83c020               	add	eax, 0x20
;;   26:	 4429d8               	sub	eax, r11d
;;   29:	 4883c410             	add	rsp, 0x10
;;   2d:	 5d                   	pop	rbp
;;   2e:	 c3                   	ret	

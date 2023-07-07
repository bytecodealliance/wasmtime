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
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 0fbdc0               	bsr	eax, eax
;;   18:	 41bb00000000         	mov	r11d, 0
;;   1e:	 410f95c3             	setne	r11b
;;   22:	 f7d8                 	neg	eax
;;   24:	 83c020               	add	eax, 0x20
;;   27:	 4429d8               	sub	eax, r11d
;;   2a:	 4883c410             	add	rsp, 0x10
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	

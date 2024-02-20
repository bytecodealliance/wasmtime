;;! target = "x86_64"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.clz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872b000000         	ja	0x46
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 0fbdc0               	bsr	eax, eax
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f95c3             	setne	r11b
;;      	 f7d8                 	neg	eax
;;      	 83c020               	add	eax, 0x20
;;      	 4429d8               	sub	eax, r11d
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   46:	 0f0b                 	ud2	

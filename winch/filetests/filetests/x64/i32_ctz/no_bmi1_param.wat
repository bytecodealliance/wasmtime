;;! target = "x86_64"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.ctz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8726000000         	ja	0x3e
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 0fbcc0               	bsf	eax, eax
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f94c3             	sete	r11b
;;      	 41c1e305             	shl	r11d, 5
;;      	 4401d8               	add	eax, r11d
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3e:	 0f0b                 	ud2	

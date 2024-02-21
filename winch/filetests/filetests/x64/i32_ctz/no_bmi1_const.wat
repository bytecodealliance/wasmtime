;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 1)
        (i32.ctz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872c000000         	ja	0x4a
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b801000000           	mov	eax, 1
;;      	 0fbcc0               	bsf	eax, eax
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f94c3             	sete	r11b
;;      	 41c1e305             	shl	r11d, 5
;;      	 4401d8               	add	eax, r11d
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4a:	 0f0b                 	ud2	

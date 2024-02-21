;;! target = "x86_64"

(module
    (func (result i32)
      i32.const 15
      i32.popcnt
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f874e000000         	ja	0x6c
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b80f000000           	mov	eax, 0xf
;;      	 89c1                 	mov	ecx, eax
;;      	 c1e801               	shr	eax, 1
;;      	 81e055555555         	and	eax, 0x55555555
;;      	 29c1                 	sub	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 41bb33333333         	mov	r11d, 0x33333333
;;      	 4421d8               	and	eax, r11d
;;      	 c1e902               	shr	ecx, 2
;;      	 4421d9               	and	ecx, r11d
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 c1e804               	shr	eax, 4
;;      	 01c8                 	add	eax, ecx
;;      	 81e00f0f0f0f         	and	eax, 0xf0f0f0f
;;      	 69c001010101         	imul	eax, eax, 0x1010101
;;      	 c1e818               	shr	eax, 0x18
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6c:	 0f0b                 	ud2	

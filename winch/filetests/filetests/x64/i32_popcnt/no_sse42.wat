;;! target = "x86_64"
;;! flags = ["has_popcnt"]

(module
    (func (result i32)
      i32.const 3
      i32.popcnt
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8745000000         	ja	0x5d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b803000000           	mov	eax, 3
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
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5d:	 0f0b                 	ud2	

;;! target = "x86_64"

(module
    (func (result i32)
      i32.const 15
      i32.popcnt
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b80f000000           	mov	eax, 0xf
;;   11:	 89c1                 	mov	ecx, eax
;;   13:	 c1e801               	shr	eax, 1
;;   16:	 81e055555555         	and	eax, 0x55555555
;;   1c:	 29c1                 	sub	ecx, eax
;;   1e:	 89c8                 	mov	eax, ecx
;;   20:	 41bb33333333         	mov	r11d, 0x33333333
;;   26:	 4421d8               	and	eax, r11d
;;   29:	 c1e902               	shr	ecx, 2
;;   2c:	 4421d9               	and	ecx, r11d
;;   2f:	 01c1                 	add	ecx, eax
;;   31:	 89c8                 	mov	eax, ecx
;;   33:	 c1e804               	shr	eax, 4
;;   36:	 01c8                 	add	eax, ecx
;;   38:	 81e00f0f0f0f         	and	eax, 0xf0f0f0f
;;   3e:	 69c001010101         	imul	eax, eax, 0x1010101
;;   44:	 c1e818               	shr	eax, 0x18
;;   47:	 4883c408             	add	rsp, 8
;;   4b:	 5d                   	pop	rbp
;;   4c:	 c3                   	ret	

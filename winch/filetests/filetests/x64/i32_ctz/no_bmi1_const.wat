;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 1)
        (i32.ctz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 0fbcc0               	bsf	eax, eax
;;   14:	 41bb00000000         	mov	r11d, 0
;;   1a:	 410f94c3             	sete	r11b
;;   1e:	 41c1e305             	shl	r11d, 5
;;   22:	 4401d8               	add	eax, r11d
;;   25:	 4883c408             	add	rsp, 8
;;   29:	 5d                   	pop	rbp
;;   2a:	 c3                   	ret	

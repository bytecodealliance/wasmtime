;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 1)
        (i32.clz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 0fbdc0               	bsr	eax, eax
;;   14:	 41bb00000000         	mov	r11d, 0
;;   1a:	 410f95c3             	setne	r11b
;;   1e:	 f7d8                 	neg	eax
;;   20:	 83c020               	add	eax, 0x20
;;   23:	 4429d8               	sub	eax, r11d
;;   26:	 4883c408             	add	rsp, 8
;;   2a:	 5d                   	pop	rbp
;;   2b:	 c3                   	ret	

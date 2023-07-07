;;! target = "x86_64"

(module
    (func (result i64)
        (local $foo i64)

        (i64.const 2)
        (local.set $foo)

        (local.get $foo)
        (i64.ctz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 48c7c002000000       	mov	rax, 2
;;   1c:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   21:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   26:	 480fbcc0             	bsf	rax, rax
;;   2a:	 41bb00000000         	mov	r11d, 0
;;   30:	 410f94c3             	sete	r11b
;;   34:	 49c1e306             	shl	r11, 6
;;   38:	 4c01d8               	add	rax, r11
;;   3b:	 4883c410             	add	rsp, 0x10
;;   3f:	 5d                   	pop	rbp
;;   40:	 c3                   	ret	

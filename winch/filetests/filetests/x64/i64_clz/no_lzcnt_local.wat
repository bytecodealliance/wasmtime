;;! target = "x86_64"

(module
    (func (result i64)
        (local $foo i64)

        (i64.const 2)
        (local.set $foo)

        (local.get $foo)
        (i64.clz)
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
;;   26:	 480fbdc0             	bsr	rax, rax
;;   2a:	 41bb00000000         	mov	r11d, 0
;;   30:	 410f95c3             	setne	r11b
;;   34:	 48f7d8               	neg	rax
;;   37:	 4883c040             	add	rax, 0x40
;;   3b:	 4c29d8               	sub	rax, r11
;;   3e:	 4883c410             	add	rsp, 0x10
;;   42:	 5d                   	pop	rbp
;;   43:	 c3                   	ret	

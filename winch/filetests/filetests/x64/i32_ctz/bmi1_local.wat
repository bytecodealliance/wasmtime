;;! target = "x86_64"
;;! flags = ["has_bmi1"]

(module
    (func (result i32)
        (local $foo i32)

        (i32.const 2)
        (local.set $foo)

        (local.get $foo)
        (i32.ctz)
    )
)

;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 b802000000           	mov	eax, 2
;;   1b:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1f:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   23:	 f30fbcc0             	tzcnt	eax, eax
;;   27:	 4883c410             	add	rsp, 0x10
;;   2b:	 5d                   	pop	rbp
;;   2c:	 c3                   	ret	

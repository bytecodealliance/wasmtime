;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)  
        (local $bar i32)

        (i32.const 1)
        (local.set $foo)

        (i32.const 2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.shr_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b801000000           	mov	eax, 1
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 b802000000           	mov	eax, 2
;;   23:	 89442408             	mov	dword ptr [rsp + 8], eax
;;   27:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   2b:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2f:	 d3e8                 	shr	eax, cl
;;   31:	 4883c410             	add	rsp, 0x10
;;   35:	 5d                   	pop	rbp
;;   36:	 c3                   	ret	

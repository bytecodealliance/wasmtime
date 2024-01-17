;;! target = "x86_64"
(module
  (func (export "as-local-set-value") (result i32)
    (local i32) (local.set 0 (loop (result i32) (i32.const 1))) (local.get 0)
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	

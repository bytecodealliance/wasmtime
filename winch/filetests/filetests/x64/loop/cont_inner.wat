;;! target = "x86_64"
(module
  (func (export "cont-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (loop (result i32) (br 1)))))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (i32.ctz (br 0)))))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (i32.ctz (loop (result i32) (br 1))))))
    (local.get 0)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b800000000           	mov	eax, 0
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   23:	 4883ec04             	sub	rsp, 4
;;   27:	 44891c24             	mov	dword ptr [rsp], r11d
;;   2b:	 e9fbffffff           	jmp	0x2b
;;   30:	 4883c410             	add	rsp, 0x10
;;   34:	 5d                   	pop	rbp
;;   35:	 c3                   	ret	

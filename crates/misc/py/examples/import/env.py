def callback(msg_p: 'i32', msg_len: 'i32') -> 'i32':
    print('callback:', msg_p, msg_len)

#    global memory
#    mv = memoryview(memory)

#    msg = bytes(mv[msg_p:(msg_p + msg_len)]).decode('utf-8')
#    print(msg)

    return 42

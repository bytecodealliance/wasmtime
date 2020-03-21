import demo

def callback(msg_p: 'i32', msg_len: 'i32') -> 'i32':
    mv = memoryview(demo.memory)
    msg = bytes(mv[msg_p:(msg_p + msg_len)]).decode('utf-8')

    print(msg)
    return 42

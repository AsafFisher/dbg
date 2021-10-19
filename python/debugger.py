from dataclasses import dataclass
import structs
import socket
import struct
from contextlib import contextmanager

@dataclass(frozen=True)
class Address:
    MACHINE_SIZE = 8
    ptr: int
    socket: socket.socket
    def __call__(self, *args):
        call_cmd_buff = structs.CallCmd(self.ptr, args).bincode_serialize()
        sock.send(structs.CMD__CALL().bincode_serialize())
        sock.send(struct.pack('Q', len(call_cmd_buff)))
        sock.send(call_cmd_buff)
        return struct.unpack('Q', sock.recv(Address.MACHINE_SIZE))

    def __add__(self, val):
        return Address(self.ptr + val, socket) 
    
    def __iadd__(self, val):
        self.ptr += val
        return self
    
    def __sub__(self, val):
        return self.__add__(self, -val)

    def __isub__(self, val):
        return self.__iadd__(self, -val)
        
    def read(self, size):
        sock.send(structs.CMD__READ().bincode_serialize())
        sock.send(structs.ReadCmd(size, self.ptr).bincode_serialize())
        return sock.recv(size)
    
    def write(self, buffer):
        write_cmd_buff = structs.WriteCmd(self.ptr, buffer).bincode_serialize()
        sock.send(structs.CMD__WRITE().bincode_serialize())
        sock.send(struct.pack('Q', len(write_cmd_buff)))
        sock.send(write_cmd_buff)

def debugger_connect():
    sock =socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(("127.0.0.1", 8080))
    return sock

sock = debugger_connect()
import pdb; pdb.set_trace()

print(addr(123))
print(addr.read(10))
#addr.write(b"hello")
print(addr.read(10))



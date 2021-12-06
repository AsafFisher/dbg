from dataclasses import dataclass
import socket
import struct
import json
from contextlib import contextmanager

@dataclass(frozen=True)
class Address:
    MACHINE_SIZE = 8
    socket: socket.socket
    ptr: int
    
    def __call__(self, *args):
        # call_cmd_buff = structs.CallCmd(self.ptr, args).bincode_serialize()
        # sock.send(structs.CMD__CALL().bincode_serialize())
        # sock.send(struct.pack('Q', len(call_cmd_buff)))
        # sock.send(call_cmd_buff)
        # return struct.unpack('Q', sock.recv(Address.MACHINE_SIZE))
        pass

    def __add__(self, val):
        return Address(self.ptr + val, socket) 
    
    def __iadd__(self, val):
        self.ptr += val
        return self
    
    def __sub__(self, val):
        return self.__add__(-val)

    def __isub__(self, val):
        return self.__iadd__(-val)
    def send_msg(self, buff):
        sock.send(struct.pack("Q", len(buff)))
        print(len(buff))
        sock.send(buff)

    def read(self, size):
        import struct
        sock.send(struct.pack("I", 0))
        sock.send(struct.pack("Q", size))
        sock.send(struct.pack("q", self.ptr))
        return sock.recv(size)

    def write(self, buffer):
        sock.send(struct.pack("I", 1))
        self.send_msg(struct.pack("q", self.ptr))
        self.send_msg(buffer)
        # sock.send(buffer)
        # write_cmd_buff = structs.WriteCmd(self.ptr, buffer).bincode_serialize()
        # sock.send(structs.CMD__WRITE().bincode_serialize())
        # sock.send(struct.pack('Q', len(write_cmd_buff)))
        # sock.send(write_cmd_buff)
        pass

def debugger_connect():
    sock =socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(("127.0.0.1", 12343))
    return sock

sock = debugger_connect()
addr = Address(sock, eval(input()))
addr.write(b'bddd')
import pdb; pdb.set_trace()


# print(addr(123))
# print(addr.read(10))
# #addr.write(b"hello")
# print(addr.read(10))



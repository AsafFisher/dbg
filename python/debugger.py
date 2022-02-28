from dataclasses import dataclass
import socket
import struct
import json
from contextlib import contextmanager

@dataclass(frozen=True)
class RemoteAddress:
    MACHINE_SIZE = 8
    sock: socket.socket
    ptr: int
    
    def __call__(self, *args: int):
        self.sock.send(struct.pack("I", 2))
        self.sock.send(struct.pack("q", self.ptr))
        self.sock.send(struct.pack("q", len(args)))
        for argument in args:
            self.sock.send(struct.pack("q", argument))
        # call_cmd_buff = structs.CallCmd(self.ptr, args).bincode_serialize()
        # sock.send(structs.CMD__CALL().bincode_serialize())
        # sock.send(struct.pack('Q', len(call_cmd_buff)))
        # sock.send(call_cmd_buff)
        # return struct.unpack('Q', sock.recv(Address.MACHINE_SIZE))
        pass

    def __add__(self, val: int):
        return Address(self.ptr + val, socket) 
    
    def __iadd__(self, val: int):
        self.ptr += val
        return self
    
    def __sub__(self, val: int):
        return self.__add__(-val)

    def __isub__(self, val: int):
        return self.__iadd__(-val)
    def send_msg(self, buff: bytes):
        self.sock.send(struct.pack("Q", len(buff)))
        print(len(buff))
        self.sock.send(buff)

    def read(self, size: int):
        import struct
        self.sock.send(struct.pack("I", 0))
        self.sock.send(struct.pack("Q", size))
        self.sock.send(struct.pack("q", self.ptr))
        return self.sock.recv(size)

    def write(self, buffer: bytes):
        self.sock.send(struct.pack("I", 1))
        self.send_msg(struct.pack("q", self.ptr))
        self.send_msg(buffer)
        # sock.send(buffer)
        # write_cmd_buff = structs.WriteCmd(self.ptr, buffer).bincode_serialize()
        # sock.send(structs.CMD__WRITE().bincode_serialize())
        # sock.send(struct.pack('Q', len(write_cmd_buff)))
        # sock.send(write_cmd_buff)
        pass



class RemoteProcess:
    """
    General Purpose Processing Unit
    """
    def __init__(self, addr: str, port: int) -> None:
        sock =socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((addr, port))
        self.socket = sock

    def leak(self, address: int) -> RemoteAddress:
        return RemoteAddress(self.socket, address)

proc = RemoteProcess("127.0.0.1", 12343)
addr = proc.leak(eval(input()))
import pdb; pdb.set_trace()
addr.write(b'bddd')
import pdb; pdb.set_trace()


# print(addr(123))
# print(addr.read(10))
# #addr.write(b"hello")
# print(addr.read(10))



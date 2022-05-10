from dataclasses import dataclass
import socket
import struct
import json
from contextlib import contextmanager
import cbor2

@dataclass(frozen=True)
class RemoteAddress:
    MACHINE_SIZE = 8
    sock: socket.socket
    ptr: int
    
    def __call__(self, *args: int):
        self.sock.send(struct.pack("I", 2))
        self.send_msg(cbor2.dumps([self.ptr, args]))
        # call_cmd_buff = structs.CallCmd(self.ptr, args).bincode_serialize()
        # sock.send(structs.CMD__CALL().bincode_serialize())
        # sock.send(struct.pack('Q', len(call_cmd_buff)))
        # sock.send(call_cmd_buff)
        # return struct.unpack('Q', sock.recv(Address.MACHINE_SIZE))
        return cbor2.loads(self.recv_msg())

    def __add__(self, val: int):
        return RemoteAddress(self.ptr + val, socket) 
    
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

    def recv_msg(self):
        amount_to_read, = struct.unpack("Q", self.sock.recv(8))
        return self.sock.recv(amount_to_read)
        

    def read(self, size: int):
        import struct
        self.sock.send(struct.pack("I", 0))
        self.send_msg(cbor2.dumps([self.ptr, size]))
        return cbor2.loads(self.recv_msg())

    def write(self, buffer: bytes):
        self.sock.send(struct.pack("I", 1))
        self.send_msg(cbor2.dumps([self.ptr, buffer]))
        # sock.send(buffer)
        # write_cmd_buff = structs.WriteCmd(self.ptr, buffer).bincode_serialize()
        # sock.send(structs.CMD__WRITE().bincode_serialize())
        # sock.send(struct.pack('Q', len(write_cmd_buff)))
        # sock.send(write_cmd_buff)
        return cbor2.loads(self.recv_msg())



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
import ipdb;ipdb.set_trace()

addr = proc.leak(eval(input()))
import ipdb;ipdb.set_trace()
addr.read(3)
import pdb; pdb.set_trace()


# print(addr(123))
# print(addr.read(10))
# #addr.write(b"hello")
# print(addr.read(10))



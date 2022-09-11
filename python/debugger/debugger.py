from dataclasses import dataclass
from socket import socket, AF_INET, SOCK_STREAM
import struct
import cbor2

from .utils import list_to_struct


@dataclass
class CallResponse:
    return_value: int


@dataclass
class ReadResponse:
    raw_data: bytes


@dataclass
class WriteResponse:
    amount_written: int


@dataclass
class Response:
    type: int
    data: CallResponse | ReadResponse | WriteResponse


@dataclass
class StatusResponseWrapper:
    status: bool
    data: Response

    def is_success(self):
        return self.status == False


class OperationFailed(Exception):
    pass


type_to_response_map = {0: ReadResponse, 1: WriteResponse, 2: CallResponse}


def dbg_type_specifier(instance, field):
    if isinstance(instance, Response):
        return type_to_response_map[instance.type]
    return None


def send_msg(sock, buff: bytes):
    sock.send(struct.pack("Q", len(buff)))
    # print(len(buff))
    sock.send(buff)


def recv_msg(sock: socket):
    (amount_to_read,) = struct.unpack("Q", sock.recv(8))
    return sock.recv(amount_to_read)


@dataclass(frozen=True)
class RemoteAddress:
    MACHINE_SIZE = 8
    sock: socket
    ptr: int

    def get_response_if_succeed(self):
        response = list_to_struct(
            cbor2.loads(recv_msg(self.sock)), StatusResponseWrapper, dbg_type_specifier
        )
        if not response.is_success():
            raise OperationFailed()
        return response.data.data

    def __call__(self, *args: int):
        self.sock.send(struct.pack("I", 2))
        send_msg(self.sock, cbor2.dumps([self.ptr, args]))
        return self.get_response_if_succeed().return_value

    def __add__(self, val: int):
        return RemoteAddress(self.ptr + val, socket)

    def __iadd__(self, val: int):
        self.ptr += val
        return self

    def __sub__(self, val: int):
        return self.__add__(-val)

    def __isub__(self, val: int):
        return self.__iadd__(-val)

    def read(self, size: int):

        self.sock.send(struct.pack("I", 0))
        send_msg(self.sock, cbor2.dumps([self.ptr, size]))
        return self.get_response_if_succeed().raw_data

    def write(self, buffer: bytes):
        self.sock.send(struct.pack("I", 1))
        send_msg(self.sock, cbor2.dumps([self.ptr, buffer]))
        return self.get_response_if_succeed().amount_written


class RemoteProcess:
    """
    General Purpose Processing Unit
    """

    def __init__(self, addr: str, port: int) -> None:
        sock = socket(AF_INET, SOCK_STREAM)
        sock.connect((addr, port))
        self.socket = sock

    def leak(self, address: int) -> RemoteAddress:
        return RemoteAddress(self.socket, address)

    def disconnect(self):
        self.socket.send(struct.pack("I", 3))
        send_msg(self.socket, b"")
        return cbor2.loads(recv_msg(self.socket))

    def shutdown(self):
        self.socket.send(struct.pack("I", 4))
        send_msg(self.socket, b"")
        return cbor2.loads(recv_msg(self.socket))

from ast import Call
from audioop import add
from dataclasses import dataclass
from socket import socket, AF_INET, SOCK_STREAM
import struct
from threading import Thread
from typing import Callable, List
import cbor2

from .utils import list_to_struct, get_fn_param_count


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
class HookInstalledResponse:
    pass


@dataclass
class Response:
    type: int
    data: CallResponse | ReadResponse | WriteResponse | HookInstalledResponse


@dataclass
class StatusResponseWrapper:
    status: bool
    data: Response | str

    def is_success(self):
        return self.status == False


# TODO: HOOK submessages


class OperationFailed(Exception):
    pass


type_to_response_map = {
    0: ReadResponse,
    1: WriteResponse,
    2: CallResponse,
    5: HookInstalledResponse,
}


def dbg_type_specifier(instance, field):
    if isinstance(instance, Response):
        return type_to_response_map[instance.type]
    if isinstance(instance, StatusResponseWrapper):
        return str if instance.status == 1 else Response
    return None


def send_msg(sock, buff: bytes):
    sock.send(struct.pack("Q", len(buff)))
    # print(len(buff))
    sock.send(buff)


def recv_msg(sock: socket):
    length_data = sock.recv(8)
    if len(length_data) == 0:
        raise Exception("Socket closed!")

    (amount_to_read,) = struct.unpack("Q", length_data)
    data = sock.recv(amount_to_read)
    if len(data) == 0:
        raise Exception("Socket closed!")
    return data


@dataclass(frozen=True)
class Hook:
    address: int
    func: Callable
    hook_thread: Thread


def handle_hook(port, hook_func):
    with socket(AF_INET, SOCK_STREAM) as hook_soc:
        hook_soc.connect(("127.0.0.1", port))
        while True:
            try:
                argz = cbor2.loads(recv_msg(hook_soc))

                # Argz is an optional in the rust endpoint, so if the len is zero, its like a none.
                # In this case it should and can never be none.
                argz = argz[0]

                def original_func(*args):
                    original_func.original_called = True
                    send_msg(
                        hook_soc, cbor2.dumps([list(args) + argz[len(args) :], True])
                    )
                    return_value = cbor2.loads(recv_msg(hook_soc))
                    return return_value[0]

                original_func.original_called = False

                # -1 because the first argument of hook_func is the original_func
                ret_val = hook_func(
                    original_func, *argz[: get_fn_param_count(hook_func) - 1]
                )

                # If original was not called we need to tell the debugger that it should not execute the original
                # function
                if not original_func.original_called:
                    send_msg(hook_soc, cbor2.dumps([[], False]))

                # Now return the ret val
                send_msg(hook_soc, cbor2.dumps([ret_val]))

            except Exception as ex:
                print(ex)


class HookPool:
    hooks: List[Hook]

    def __init__(self) -> None:
        self.hooks = []

    def add_hook(self, address: int, hook_func: Callable, port) -> bool:
        if any(hook.address == address for hook in self.hooks):
            raise Exception("Hook already exists")
        hook_thread = Thread(target=handle_hook, args=(port, hook_func))
        hook_thread.start()
        self.hooks.append(Hook(address, hook_func, hook_thread))


@dataclass(frozen=True)
class RemoteAddress:
    MACHINE_SIZE = 8
    sock: socket
    ptr: int
    hook_poll: HookPool

    def get_response_if_succeed(self):
        response = list_to_struct(
            cbor2.loads(recv_msg(self.sock)), StatusResponseWrapper, dbg_type_specifier
        )

        if not response.is_success():
            raise OperationFailed(response.data)

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

    def hook(self, prefix_size: int, hook_func: Callable):
        self.sock.send(struct.pack("I", 5))
        port = 5555
        send_msg(self.sock, cbor2.dumps([self.ptr, prefix_size, port]))

        import time

        time.sleep(1)

        self.hook_poll.add_hook(self.ptr, hook_func, port)
        self.get_response_if_succeed()


class RemoteProcess:
    """
    General Purpose Processing Unit
    """

    def __init__(self, addr: str, port: int) -> None:
        sock = socket(AF_INET, SOCK_STREAM)
        sock.connect((addr, port))
        self.socket = sock
        self.hook_pool = HookPool()

    def leak(self, address: int) -> RemoteAddress:
        return RemoteAddress(self.socket, address, self.hook_pool)

    def disconnect(self):
        self.socket.send(struct.pack("I", 3))
        send_msg(self.socket, b"")
        return cbor2.loads(recv_msg(self.socket))

    def shutdown(self):
        self.socket.send(struct.pack("I", 4))
        send_msg(self.socket, b"")
        return cbor2.loads(recv_msg(self.socket))

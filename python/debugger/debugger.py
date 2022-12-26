from ast import Call
from audioop import add
from dataclasses import dataclass
from socket import socket, AF_INET, SOCK_STREAM
import struct
from threading import Thread
from typing import Callable, List
from contextlib import contextmanager
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
class HookToggledResponse:
    pass


@dataclass
class Response:
    type: int
    data: CallResponse | ReadResponse | WriteResponse | HookInstalledResponse | HookToggledResponse


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
    6: HookToggledResponse,
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


def get_response_if_succeed(sock: socket):
    response = list_to_struct(
        cbor2.loads(recv_msg(sock)), StatusResponseWrapper, dbg_type_specifier
    )

    if not response.is_success():
        raise OperationFailed(response.data)

    return response.data.data


@dataclass
class Hook:
    address: int
    func: Callable
    hook_thread: Thread
    command_sock: socket

    # Toggle on or off the hook
    def toggle(self, enabled):
        self.command_sock.send(struct.pack("I", 6))
        send_msg(self.command_sock, cbor2.dumps([self.address, enabled]))
        return get_response_if_succeed(self.command_sock)

    # Enable the hook
    def enable(self):
        self.toggle(True)

    # Disable the hook
    def disable(self):
        self.toggle(False)

    @contextmanager
    def enabled(self):
        self.toggle(True)
        try:
            yield
        except Exception as ex:
            raise ex
        finally:
            self.toggle(False)


class HookPool:
    # TODO: remove hook
    hooks: List[Hook]

    def __init__(self) -> None:
        self.hooks = []

    def add_hook(
        self,
        address: int,
        hook_func: Callable,
        port: int,
        sock: socket,
    ) -> Hook:
        if any(hook.address == address for hook in self.hooks):
            raise Exception("Hook already exists")
        
        # Creates a hook thread. It handles the comm with the debugger on hook context.
        hook_thread = Thread(target=handle_hook)
        hook = Hook(address, hook_func, hook_thread, sock)
        
        # We do this because we want to pass the hook object into the thread so 
        # when the hook's pyhton handling function changes, the thread will access the new one
        # through the hook object
        hook_thread._args=(port, hook)
        
        # Start handling hooks
        hook_thread.start()
        
        self.hooks.append(hook)
        return hook


@dataclass(frozen=True)
class RemoteAddress:
    MACHINE_SIZE = 8
    sock: socket
    ptr: int
    hook_poll: HookPool

    def __call__(self, *args: int):
        self.sock.send(struct.pack("I", 2))
        send_msg(self.sock, cbor2.dumps([self.ptr, args]))
        return get_response_if_succeed(self.sock).return_value

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
        return get_response_if_succeed(self.sock).raw_data

    def write(self, buffer: bytes):
        self.sock.send(struct.pack("I", 1))
        send_msg(self.sock, cbor2.dumps([self.ptr, buffer]))
        return get_response_if_succeed(self.sock).amount_written

    def hook(self, prefix_size: int, hook_func: Callable):
        existing_hook = [i for i, hook in  enumerate(self.hook_poll.hooks) if hook.address == self.ptr]
        # If a hook already exists, we just want to replace the hook_func, there is no need
        # to recreate all the handling threads, etc...
        if len(existing_hook) > 0:
            self.hook_poll.hooks[existing_hook[0]].func = hook_func
            return self.hook_poll.hooks[existing_hook[0]]
        self.sock.send(struct.pack("I", 5))
        # TODO: allocate port automatically
        port = 5555
        send_msg(self.sock, cbor2.dumps([self.ptr, prefix_size, port]))

        import time

        time.sleep(1)

        hook = self.hook_poll.add_hook(self.ptr, hook_func, port, self.sock)
        get_response_if_succeed(self.sock)
        return hook


def handle_hook(port, hook: Hook):
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
                ret_val = hook.func(
                    original_func, *argz[: get_fn_param_count(hook.func) - 1]
                )

                # If original was not called we need to tell the debugger that it should not execute the original
                # function
                if not original_func.original_called:
                    send_msg(hook_soc, cbor2.dumps([[], False]))

                # Now return the ret val
                send_msg(hook_soc, cbor2.dumps([ret_val]))

            except Exception as ex:
                print(ex)


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

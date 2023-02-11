from __future__ import annotations

from contextlib import contextmanager
from dataclasses import dataclass
from threading import Thread
from typing import Callable, Generator, List

from debugger_core import DebugController, HookController

from .utils import get_fn_param_count


@dataclass
class Hook:
    address: int
    func: Callable
    hook_thread: Thread
    conn: DebugController

    # Toggle on or off the hook
    def toggle(self, enabled):
        return self.conn.hook_toggle(self.address, enabled)

    # Enable the hook
    def enable(self) -> None:
        self.toggle(True)

    # Disable the hook
    def disable(self) -> None:
        self.toggle(False)

    @contextmanager
    def enabled(self) -> Generator[None, None, None]:
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
        conn: DebugController,
    ) -> Hook:
        if any(hook.address == address for hook in self.hooks):
            raise Exception("Hook already exists")

        # Creates a hook thread. It handles the comm with the debugger on hook context.
        hook_thread = Thread(target=handle_hook)
        hook = Hook(address, hook_func, hook_thread, conn)

        # We do this because we want to pass the hook object into the thread so
        # when the hook's pyhton handling function changes, the thread will access the new one
        # through the hook object
        hook_thread._args = (port, hook)  # type: ignore

        # Start handling hooks
        hook_thread.start()

        self.hooks.append(hook)
        return hook


@dataclass
class RemoteAddress:
    MACHINE_SIZE = 8
    conn: DebugController
    ptr: int
    hook_poll: HookPool

    def __call__(self, *args: int):
        return self.conn.call(self.ptr, args)

    def __add__(self, val: int) -> RemoteAddress:
        return RemoteAddress(self.conn, self.ptr + val, self.hook_poll)

    def __iadd__(self, val: int) -> RemoteAddress:
        self.ptr += val
        return self

    def __sub__(self, val: int) -> RemoteAddress:
        return self.__add__(-val)

    def __isub__(self, val: int) -> RemoteAddress:
        return self.__iadd__(-val)

    def read(self, size: int) -> bytes:
        return self.conn.read(self.ptr, size)

    def write(self, buffer: bytes) -> int:
        return self.conn.write(self.ptr, buffer)

    def hook(self, prefix_size: int, hook_func: Callable) -> Hook:
        existing_hook = [
            i for i, hook in enumerate(self.hook_poll.hooks) if hook.address == self.ptr
        ]
        # If a hook already exists, we just want to replace the hook_func, there is no need
        # to recreate all the handling threads, etc...
        if len(existing_hook) > 0:
            self.hook_poll.hooks[existing_hook[0]].func = hook_func
            return self.hook_poll.hooks[existing_hook[0]]

        # TODO: allocate port automatically
        port = 5555

        # Will try connect to hook on different thread
        hook = self.hook_poll.add_hook(self.ptr, hook_func, port, self.conn)

        # Tell debugger to listen
        self.conn.hook(self.ptr, prefix_size, port)

        import time

        time.sleep(1)

        return hook


def handle_hook(port: int, hook: Hook) -> None:
    with HookController(
        "127.0.0.1" + ":" + str(port), retries=30, interval=0.1
    ) as hook_conn:
        while True:
            try:
                # Get the arguments state before the function call
                argz = hook_conn.recv_precall_args()

                def original_func(*args):
                    original_func.original_called = True

                    # Call the original function with our arguments hehehe, then return the result
                    return hook_conn.call_original_with_args(
                        list(args) + argz[len(args):]
                    )

                # TODO: multiple hooks will need to have different original_func.original_called (one could effect the other)
                # so just replace it with a dict with hook.address as the key.
                original_func.original_called = False

                # -1 because the first argument of hook_func is the original_func
                ret_val = hook.func(
                    original_func, *argz[: get_fn_param_count(hook.func) - 1]
                )

                # If original was not called we need to tell the debugger that it should not execute the original
                # function
                if not original_func.original_called:
                    hook_conn.skip_original_function()

                # Now return the ret val we want to set
                hook_conn.postcall_set_retval(ret_val)

            except Exception as ex:
                print(ex)


class RemoteProcess:
    """
    General Purpose Processing Unit
    """

    def __init__(self, addr: str) -> None:
        self.conn = DebugController(addr)
        self.hook_pool = HookPool()

    def leak(self, address: int) -> RemoteAddress:
        return RemoteAddress(self.conn, address, self.hook_pool)

    def disconnect(self) -> None:
        self.conn.disconnect()

    def shutdown(self) -> None:
        self.conn.shutdown()

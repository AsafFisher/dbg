import inspect
from typing import Callable


def get_fn_param_count(function: Callable[[...], int]):
    return len(inspect.getargspec(function).args)

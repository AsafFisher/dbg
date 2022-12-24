from dataclasses import dataclass, fields, is_dataclass
from types import UnionType
from typing import get_args, Callable
import inspect


def list_to_struct(lst, struct_class, type_sprcifier=None, dynamic_condition=None):
    if not isinstance(lst, list):
        if lst == None:
            return struct_class()
        return struct_class(lst)
    instance = struct_class(*lst)
    for field in fields(struct_class):
        field_type = field.type
        if type_sprcifier and type(field_type) == UnionType:
            field_type_tmp = type_sprcifier(instance, field)
            # Check that the type is valid
            assert field_type_tmp in get_args(field_type)
            assert type(field_type_tmp) != UnionType, "Umbegiouse Union"
            field_type = field_type_tmp

            inner_lst = instance.__dict__[field.name]
            if inner_lst == []:
                instance.__dict__[field.name] = list_to_struct(
                    None, field_type, type_sprcifier
                )
            else:
                instance.__dict__[field.name] = list_to_struct(
                    *instance.__dict__[field.name], field_type, type_sprcifier
                )

        if is_dataclass(field.type):
            instance.__dict__[field.name] = list_to_struct(
                *instance.__dict__[field.name], field_type, type_sprcifier
            )
    return instance


def get_fn_param_count(function: Callable[[...], int]):
    return len(inspect.getargspec(function).args)

from dataclasses import dataclass, fields, is_dataclass
from types import UnionType
from typing import get_args


def list_to_struct(lst, struct_class, type_sprcifier=None):
    if not isinstance(lst, list):
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
            instance.__dict__[field.name] = list_to_struct(
                *instance.__dict__[field.name], field_type, type_sprcifier
            )

        if is_dataclass(field.type):
            instance.__dict__[field.name] = list_to_struct(
                *instance.__dict__[field.name], field_type, type_sprcifier
            )
    return instance

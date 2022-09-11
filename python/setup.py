#!/usr/bin/env python

from distutils.core import setup

setup(
    name="Debugger",
    version="1.0",
    description="The debugger controller",
    author="Asaf Fisher",
    packages=["debugger"],
    install_requires=[
        "cbor2==5.4.3",
    ],
)

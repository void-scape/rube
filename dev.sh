#!/bin/bash

# `prefer-dynamic` allows `rube` to dynamically link with the `rube-platform`, preventing
# any global state duplication upon loading `rube` from statically linked libraries.
RUSTFLAGS="-C prefer-dynamic" cargo run $1

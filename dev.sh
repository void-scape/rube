#!/bin/bash

# `prefer-dynamic` allows `rube` to dynamically link with `rube-core`, preventing
# any global state duplication upon loading `rube` from statically linked libraries.
RUSTFLAGS="-C prefer-dynamic" cargo run

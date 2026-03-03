#!/bin/bash

set -e

mkdir -p bench
TIME=$(date +%Y_%m_%d_%H_%M_%S)
TRACY_FILE="bench/${TIME}.tracy"
MARKDOWN_FILE="bench/${TIME}.md"

cargo build --no-default-features --features tracy --release 
tracy-capture -f -o ${TRACY_FILE} &
PID=$!
./target/release/rube-bin assets/castle.bin.bz2 > /tmp/rube-bench-frame.md
wait $PID
cat /tmp/rube-bench-frame.md > ${MARKDOWN_FILE}
echo -e "### Scope\n" >> ${MARKDOWN_FILE}
tracy-csvexport ${TRACY_FILE} | column -s, -t | sed 's/  */ | /g' | sed 's/^/| /' | sed 's/$/ |/' >> ${MARKDOWN_FILE}

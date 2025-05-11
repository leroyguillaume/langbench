#!/bin/bash

set -e

export THREADS=$(nproc)

/usr/bin/time -f "%e,%S,%U,%P,%M" -o result.csv "$@"

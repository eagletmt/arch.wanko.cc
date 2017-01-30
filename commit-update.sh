#!/bin/sh
exec cargo run --quiet --bin commit-update -- "$@"

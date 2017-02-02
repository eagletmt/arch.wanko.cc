#!/bin/sh
exec cargo run --quiet --bin submodule-diff -- "$@"

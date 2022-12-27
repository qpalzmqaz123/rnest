#!/bin/bash

set -e

# Check rust code
cargo clippy $@ -- \
    -D warnings \
    -D clippy::all \
    -D clippy::pedantic	\
    -D clippy::nursery \
    -D clippy::cargo \
    -D clippy::restriction \
    -A clippy::multiple_crate_versions \
    -A clippy::implicit_return \
    -A clippy::clone_on_ref_ptr \
    -A clippy::pattern_type_mismatch \
    -A clippy::integer_arithmetic \
    -A clippy::float_arithmetic \
    -A clippy::arithmetic-side-effects \
    -A clippy::str_to_string \
    -A clippy::exhaustive_structs \
    -A clippy::missing_docs_in_private_items \
    -A clippy::module_name_repetitions \
    -A clippy::mod_module_files \
    -A clippy::non_ascii_literal \
    -A clippy::default_numeric_fallback \
    -A clippy::map_err_ignore \
    -A clippy::std_instead_of_alloc \
    -A clippy::std_instead_of_core \
    -A clippy::rc_buffer \
    -A clippy::similar_names \
    -A clippy::unnecessary_wraps \
    -A clippy::future_not_send \

# Format rust code
cargo fmt

# Check if file modified by cargo fmt
(! (git status --porcelain=v1 | grep '^.M'))


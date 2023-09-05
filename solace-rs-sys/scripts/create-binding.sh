#!/usr/bin/env sh

# Must contain `solclient` directory with `solClient.h`, `solClientMsg.h`.
# Also requires bindgen cli to be installed
solace_include_path="$1"

bindgen=bindgen

"$bindgen" \
    --no-doc-comments \
    --with-derive-default \
    --allowlist-function '^solClient_.*' \
    --allowlist-var '^SOLCLIENT_.*' \
    --output ./src/solace_binding.rs \
    wrapper.h \
    -- -I "$solace_include_path"

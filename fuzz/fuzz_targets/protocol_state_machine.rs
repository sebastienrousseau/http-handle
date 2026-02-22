// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = http_handle::protocol_state::classify_protocol_bytes(data);
});

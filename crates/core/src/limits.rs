// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Field length limits (in Unicode characters, not bytes).

pub const TITLE_MAX: usize = 100;
pub const CATEGORY_MAX: usize = 50;
pub const SUMMARY_MAX: usize = 200;
pub const DESCRIPTION_MAX: usize = 5000;
pub const COMMAND_MAX: usize = 4000;
pub const PATH_MAX: usize = 1024;
pub const SCRIPT_ARGS_MAX: usize = 1000;
/// Default and maximum wall-clock time a job command may run.
pub const TIMEOUT_DEFAULT_SECS: u32 = 60;
pub const TIMEOUT_MAX_SECS: u32 = 86_400;
/// "Retry on failure": how many extra attempts, and the wait between them.
pub const RETRY_COUNT_DEFAULT: u32 = 3;
pub const RETRY_COUNT_MAX: u32 = 10;
pub const RETRY_DELAY_DEFAULT_SECS: u32 = 60;
pub const RETRY_DELAY_MAX_SECS: u32 = 3600;
pub const FULL_NAME_MAX: usize = 100;
pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 32;
pub const PASSWORD_MIN: usize = 10;
pub const PASSWORD_MAX: usize = 128;
/// Upper bound for the "n" in "every n <unit>" to keep arithmetic sane.
pub const REPEAT_N_MAX: u32 = 10_000;
pub const PAGE_SIZE_DEFAULT: u32 = 10;
pub const PAGE_SIZE_OPTIONS: [u32; 5] = [5, 10, 20, 50, 100];

// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

//! Field length limits (in Unicode characters, not bytes).

pub const TITLE_MAX: usize = 100;
pub const CATEGORY_MAX: usize = 50;
pub const SUMMARY_MAX: usize = 200;
pub const DESCRIPTION_MAX: usize = 5000;
pub const FULL_NAME_MAX: usize = 100;
pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 32;
pub const PASSWORD_MIN: usize = 10;
pub const PASSWORD_MAX: usize = 128;
/// Upper bound for the "n" in "every n <unit>" to keep arithmetic sane.
pub const REPEAT_N_MAX: u32 = 10_000;
pub const PAGE_SIZE_DEFAULT: u32 = 10;
pub const PAGE_SIZE_OPTIONS: [u32; 5] = [5, 10, 20, 50, 100];

/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! Shared test scaffolding: the mini host grammar hoover plugs into.
//!
//! Cargo compiles this module into EVERY integration test binary, so a
//! helper only one of them uses reads as dead code in the others. The
//! allow is about that compilation model, not about unused code.

#![allow(dead_code)]

pub mod mini_grammar;

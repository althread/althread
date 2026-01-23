//! Linear Temporal Logic (LTL) verification module.
//!
//! This module provides LTL model checking capabilities using the automata-theoretic approach:
//!
//! # Submodules
//!
//! - [`ast`]: LTL syntax tree representation (parsed from source)
//! - [`compiled`]: Compiled LTL expressions ready for verification
//! - [`automaton`]: Büchi automaton construction from LTL formulas
//! - [`monitor`]: Runtime state tracking during exploration
//! - [`evaluator`]: Predicate evaluation on VM states
//! - [`quantifier`]: Handling of `for`/`exists` quantifiers over processes
//! - [`debug`]: Diagnostic utilities (DOT export, formula formatting)
//!
//! # Usage
//!
//! The verification flow is:
//! 1. Parse LTL formulas from source → `ast`
//! 2. Compile to executable form → `compiled`
//! 3. Build Büchi automatons (negated) → `automaton`
//! 4. Explore product space with monitors → `monitor`
//! 5. Detect accepting cycles → violation found

pub mod ast;
pub mod automaton;
pub mod compiled;
pub mod debug;
pub mod evaluator;
pub mod monitor;
pub mod quantifier;

#[cfg(test)]
mod automaton_tests;

#[cfg(test)]
mod usage_examples;

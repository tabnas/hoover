/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

//! A deliberately tiny, bespoke grammar, not JSON, that provides just
//! enough structure to exercise the hoover plugin without a full grammar
//! package. hoover's only dependency is the tabnas engine; this grammar
//! lives in test code.
//!
//! ```text
//! value := scalar | '(' value ')'
//! ```
//!
//! A scalar is any built-in value token or a hoover token. Parentheses
//! group a single value, which gives the inner `val` rule a `group`
//! parent, enough to test rule-context (parent) matching. hoover
//! registers its block token as an extra `val` alternate, so the engine's
//! `val` rule is the integration point.
//!
//! This is the Rust twin of `ts/test/minigrammar.ts` and
//! `go/minigrammar_test.go`; keep the three in step.

use std::cell::RefCell;
use std::rc::Rc;

use tabnas::{AltSpec, Rule, Tabnas, Value, ValueDef, TIN_ZZ};
use tabnas_hoover::{hoover, HooverOptions};

/// Assign a rule's node. A pushed rule shares its parent's node cell, so
/// an assignment (`r.node = ...` in TypeScript) installs a fresh cell.
fn set_node(rule: &mut Rule, value: Value) {
    rule.node = Rc::new(RefCell::new(value));
}

/// Install the `val` + `group` grammar. With `tagged`, every alt carries
/// a `mini` group tag and `rule.include` is narrowed to it, the shape a
/// strict grammar plugin uses to select a subset of a richer default
/// (`@tabnas/json` does exactly this with `rule: { include: 'json' }`).
fn install(parser: &mut Tabnas, tagged: bool) {
    let tag = if tagged { "mini" } else { "" };

    parser
        .set_options(|options| {
            options.rule.start = "val".to_string();
            if tagged {
                options.rule.include = "mini".to_string();
            }
            // Define a few keyword values so value resolution is
            // deterministic.
            for (source, value) in [
                ("true", Value::Bool(true)),
                ("false", Value::Bool(false)),
                ("null", Value::Null),
            ] {
                options.value.definitions.insert(
                    source.to_string(),
                    ValueDef {
                        val: Some(value),
                        matcher: None,
                        transform: None,
                        consume: false,
                    },
                );
            }
        })
        .expect("the mini grammar options apply");

    let op = parser.token_with_source("#OP", "(");
    let cp = parser.token_with_source("#CP", ")");
    let val_set = parser
        .token_set("VAL")
        .expect("the engine default VAL token set exists");

    // val: a scalar value, or a parenthesised group.
    parser.define_rule("val", move |spec| {
        spec.add_bo(|rule, _context| {
            set_node(rule, Value::Undefined);
        });
        spec.add_bc(|rule, context| {
            if !rule.node.borrow().is_undefined() {
                return;
            }
            if !rule.child_node.is_undefined() {
                let child = rule.child_node.clone();
                set_node(rule, child);
            } else if rule.os() != 0 {
                let value = rule.resolve_open_value(0, context);
                set_node(rule, value);
            }
        });
        spec.add_open(AltSpec {
            s: vec![vec![op]],
            p: Some("group".into()),
            b: 1,
            g: tag.into(),
            ..Default::default()
        })
        .add_open(AltSpec {
            s: vec![val_set],
            g: tag.into(),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![TIN_ZZ]],
            g: tag.into(),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![cp]],
            b: 1,
            g: tag.into(),
            ..Default::default()
        });
    });

    // group: '(' value ')', yields the inner value.
    parser.define_rule("group", move |spec| {
        spec.add_bc(|rule, _context| {
            let child = rule.child_node.clone();
            set_node(rule, child);
        });
        spec.add_open(AltSpec {
            s: vec![vec![op]],
            p: Some("val".into()),
            g: tag.into(),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![cp]],
            g: tag.into(),
            ..Default::default()
        });
    });
}

/// The mini grammar: `val` + a parenthesised `group`, untagged.
pub fn mini_grammar(parser: &mut Tabnas) {
    install(parser, false);
}

/// The mini grammar with every alt tagged `mini` and `rule.include`
/// narrowed to it.
pub fn tag_grammar(parser: &mut Tabnas) {
    install(parser, true);
}

/// Build a bare engine, install the mini grammar (the dependency), then
/// the hoover plugin with the given options.
pub fn make_mini(options: HooverOptions) -> Tabnas {
    let mut parser = Tabnas::new();
    mini_grammar(&mut parser);
    hoover(&mut parser, options).expect("hoover registers on the mini grammar");
    parser
}

/// `make_mini` against `tag_grammar`.
pub fn make_tagged(options: HooverOptions) -> Tabnas {
    let mut parser = Tabnas::new();
    tag_grammar(&mut parser);
    hoover(&mut parser, options).expect("hoover registers on the tagged mini grammar");
    parser
}

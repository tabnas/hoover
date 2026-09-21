/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

// These tests run the hoover plugin against the tiny local grammar in
// tests/common/mini_grammar.rs (val + parenthesised group). The grammar
// exists only to give hoover something to plug into; hoover's only
// production dependency is the tabnas engine.
//
// They mirror ts/test/hoover.test.ts and go/hoover_test.go. Prefer a
// fixture in ../test/spec over a case here whenever the case is
// expressible as input -> output; what stays here is what a fixture
// cannot express (registration errors, the token's `use` bag, the
// typed-options surface, thread safety).

mod common;

use std::sync::{Arc, Mutex};

use common::mini_grammar::{make_mini, make_tagged, mini_grammar, tag_grammar};
use tabnas::{Tabnas, Value};
use tabnas_hoover::{
    hoover, plugin, Block, Consume, HooverError, HooverOptions, HooverRuleFilter, HooverRuleSpec,
    DEFAULT_LEX_ORDER, DEFAULT_TOKEN, VERSION,
};

fn parse_eq(parser: &Tabnas, src: &str, want: Value) {
    match parser.parse(src) {
        Ok(got) => assert_eq!(got, want, "parse({src:?})"),
        Err(error) => panic!("parse({src:?}) error: {error}"),
    }
}

fn parse_str(parser: &Tabnas, src: &str, want: &str) {
    parse_eq(parser, src, Value::String(want.to_string()));
}

fn parse_err(parser: &Tabnas, src: &str) -> tabnas::TabnasError {
    match parser.parse(src) {
        Ok(got) => panic!("parse({src:?}) = {got:?}, want an error"),
        Err(error) => error,
    }
}

fn one(block: Block) -> HooverOptions {
    HooverOptions::new(vec![block])
}

/// `'''...'''` fixed delimiters, default options.
fn triple_quote() -> Tabnas {
    make_mini(one(Block::delimited("triplequote", "'''", "'''")))
}

#[test]
fn fixed_delimiters() {
    let parser = triple_quote();
    parse_str(&parser, "'''x'''", "x");
    parse_str(&parser, "'''hello world'''", "hello world"); // spaces preserved
    parse_str(&parser, "'''a\nb'''", "a\nb"); // newlines preserved
    parse_str(&parser, "'''  spaced  '''", "  spaced  "); // no trim by default
    parse_str(&parser, "('''x''')", "x"); // nested in a group
    parse_str(&parser, "(''' a b ''')", " a b ");
}

#[test]
fn eof_and_multiple_end_delimiters() {
    // ~ opens; closes on '>', '!' or end-of-input.
    let parser = make_mini(one(Block::new("tilde")
        .with_start(&["~"])
        .with_end(&[">", "!", ""])));
    parse_str(&parser, "~hello world", "hello world"); // EOF terminates
    parse_str(&parser, "~a>", "a"); // first delimiter
    parse_str(&parser, "~a!", "a"); // second delimiter
}

#[test]
fn escapes() {
    // < ... > with backslash escapes; allow_unknown_escape defaults to true.
    let parser = make_mini(one(Block::delimited("angle", "<", ">")
        .with_escape_char('\\')
        .with_escape("n", "\n")
        .with_escape(">", ">")
        .with_escape("\\", "\\")));
    parse_str(&parser, "<a\\>b>", "a>b"); // escaped end delimiter
    parse_str(&parser, "<a\\nb>", "a\nb"); // mapped escape
    parse_str(&parser, "<a\\\\b>", "a\\b"); // escaped backslash
    parse_str(&parser, "<a\\zb>", "azb"); // unknown escape: backslash dropped
    parse_str(&parser, "<a\\\nb>", "a\nb"); // backslash before a literal newline
}

#[test]
fn reject_unknown_escape() {
    let mut block = Block::delimited("angle", "<", ">")
        .with_escape_char('\\')
        .with_escape(">", ">");
    block.allow_unknown_escape = Some(false);
    let parser = make_mini(one(block));
    parse_str(&parser, "<a\\>b>", "a>b"); // known escape still works
    let error = parse_err(&parser, "<a\\zb>"); // unknown escape rejected
    assert_eq!(error.code, "invalid_escape");
}

#[test]
fn preserve_escape_char() {
    let mut block = Block::delimited("angle", "<", ">").with_escape_char('\\');
    block.preserve_escape_char = true;
    let parser = make_mini(one(block));
    parse_str(&parser, "<a\\zb>", "a\\zb"); // escape char kept in output
}

#[test]
fn trim() {
    let mut block = Block::delimited("angle", "<", ">");
    block.trim = true;
    let parser = make_mini(one(block));
    parse_str(&parser, "<  hello  >", "hello");
    parse_str(&parser, "< a b >", "a b"); // internal spaces kept, edges trimmed
                                          // The JavaScript trim set, not Unicode White_Space: the BOM goes,
                                          // NEL stays.
    parse_str(&parser, "<\u{FEFF}hello>", "hello");
    parse_str(&parser, "<\u{0085}hello\u{0085}>", "\u{0085}hello\u{0085}");
}

#[test]
fn selective_end_consume() {
    // Closes on ';' or end-of-input; only ';' is consumed.
    let mut block = Block::new("tilde").with_start(&["~"]).with_end(&[";", ""]);
    block.end.consume = Consume::Only(vec![";".into()]);
    let parser = make_mini(one(block));
    parse_str(&parser, "~a b;", "a b");
    parse_str(&parser, "~a b", "a b");
}

#[test]
fn rule_context_parent() {
    // The @ block only matches when the current rule's parent is `group`.
    let parser = make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            parent: Some(HooverRuleFilter::include(&["group"])),
            ..Default::default()
        },
    )));
    // Inside a group (parent == group): matches.
    parse_str(&parser, "(@hello world@)", "hello world");
    // At top level (parent is not group): does not match, so the bare
    // '@' is unexpected and the parse fails.
    parse_err(&parser, "@hello world@");
}

#[test]
fn custom_token_name() {
    // A non-default token name still parses and is registered on the
    // engine.
    let mut block = Block::delimited("tq", "'''", "'''");
    block.token = Some("#XX".into());
    let mut parser = Tabnas::new();
    mini_grammar(&mut parser);
    hoover(&mut parser, one(block)).expect("hoover registers");
    parse_str(&parser, "'''x'''", "x");
    assert!(parser.config().token("#XX").is_some(), "#XX is registered");
    assert!(parser.rule_names().iter().any(|name| name == "val"));
}

#[test]
fn fail_fast_on_missing_grammar() {
    // Registering hoover on a bare engine (no grammar, no `val` rule)
    // returns a clear error instead of failing confusingly later.
    let mut parser = Tabnas::new();
    let error = hoover(&mut parser, one(Block::delimited("tq", "'''", "'''")))
        .expect_err("hoover must refuse an instance without a val rule");
    assert!(
        error.0.contains("'val' rule is missing"),
        "unexpected message: {error}"
    );
}

#[test]
fn start_delimiter_consume() {
    // consume: Never leaves the start delimiter in the value.
    let mut block = Block::delimited("a", "<", ">");
    block.start.as_mut().expect("start").consume = Consume::Never;
    let parser = make_mini(one(block));
    parse_str(&parser, "<hi>", "<hi");

    // consume: Only(...) consumes only the listed start delimiters.
    let mut block = Block::new("a").with_start(&["<", "~"]).with_end(&[">"]);
    block.start.as_mut().expect("start").consume = Consume::Only(vec!["<".into()]);
    let parser = make_mini(one(block));
    parse_str(&parser, "<hi>", "hi"); // '<' consumed
    parse_str(&parser, "~hi>", "~hi"); // '~' kept
}

#[test]
fn end_delimiter_consume() {
    // consume: Never leaves the end delimiter, here the group's ')'.
    let mut block = Block::delimited("a", "~", ")");
    block.end.consume = Consume::Never;
    let parser = make_mini(one(block));
    parse_str(&parser, "(~hi)", "hi");

    // consume: All removes the end delimiter.
    let mut block = Block::new("a").with_start(&["~"]).with_end(&[";", ""]);
    block.end.consume = Consume::All;
    let parser = make_mini(one(block));
    parse_str(&parser, "~hi;", "hi");
}

#[test]
fn rule_context_parent_exclude() {
    let parser = make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            parent: Some(HooverRuleFilter::exclude(&["group"])),
            ..Default::default()
        },
    )));
    parse_str(&parser, "@hi@", "hi"); // top level: parent not group, matches
    parse_str(&parser, "(@hi@)", "@hi@"); // inside group: excluded, text token
}

#[test]
fn rule_context_current_filter() {
    let parser = make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            current: Some(HooverRuleFilter {
                include: Some(vec!["val".into()]),
                exclude: Some(vec!["group".into()]),
            }),
            ..Default::default()
        },
    )));
    parse_str(&parser, "@hi@", "hi");

    // Excluding the current rule turns the block off everywhere.
    let parser = make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            current: Some(HooverRuleFilter::exclude(&["val"])),
            ..Default::default()
        },
    )));
    parse_str(&parser, "@hi@", "@hi@");
}

fn at_with_state(state: &str, parent: Option<HooverRuleFilter>) -> Tabnas {
    make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            parent,
            current: None,
            state: Some(state.into()),
        },
    )))
}

#[test]
fn rule_context_state() {
    // explicit state 'oc' checks open|close; matches at val open.
    let parser = at_with_state("oc", None);
    parse_str(&parser, "@hi@", "hi");

    // explicit state 'o' is the default and matches at val open.
    let parser = at_with_state("o", None);
    parse_str(&parser, "@hi@", "hi");

    // state 'c' only: the val rule is OPEN when the block would start,
    // so the block never matches and the source lexes as text.
    let parser = at_with_state("c", None);
    parse_str(&parser, "@hi@", "@hi@");

    // state '' with a parent filter: the state check is skipped and the
    // parent filter decides.
    let parser = at_with_state("", Some(HooverRuleFilter::include(&["group"])));
    parse_str(&parser, "(@hi@)", "hi");
    parse_str(&parser, "@hi@", "@hi@"); // top level: one text token
    parse_err(&parser, "@hello world@"); // top level: two, so unexpected

    // state '' with NO parent/current filter still matches: turning the
    // state check off leaves no rule condition at all, which is no
    // constraint, not a failed one.
    let parser = at_with_state("", None);
    parse_str(&parser, "@hi@", "hi");
    parse_str(&parser, "(@hi@)", "hi");
}

#[test]
fn escape_char_as_the_final_source_character() {
    // Nothing to escape: the block never reaches its end delimiter (not
    // even the configured EOF one) and is reported as unterminated.
    let parser = make_mini(one(Block::new("a")
        .with_start(&["~"])
        .with_end(&[">", ""])
        .with_escape_char('\\')));
    parse_str(&parser, "~ab>", "ab"); // control: terminates normally
    let error = parse_err(&parser, "~ab\\");
    assert_eq!(error.code, "invalid_text");
}

#[test]
fn unterminated_block_is_invalid_text() {
    let parser = triple_quote();
    let error = parse_err(&parser, "'''never closed");
    assert_eq!(error.code, "invalid_text");
}

#[test]
fn newline_terminated_value() {
    let parser = make_mini(one(Block::new("line")
        .with_start(&["~"])
        .with_end(&["\n", "\r\n", ""])));
    parse_str(&parser, "~a b\n", "a b"); // newline consumed
    parse_str(&parser, "~a b\r\n", "a b"); // CRLF consumed
    parse_str(&parser, "~a b", "a b"); // EOF
}

#[test]
fn resolves_keyword_values() {
    // A hoovered value that matches a defined value (true/false/null)
    // resolves to that value, not the string.
    let parser = make_mini(one(Block::new("a").with_start(&["~"]).with_end(&[">", ""])));
    parse_eq(&parser, "~true>", Value::Bool(true));
    parse_eq(&parser, "~null>", Value::Null);
    parse_str(&parser, "~hello>", "hello"); // non-keyword stays a string
}

#[test]
fn registers_without_an_explicit_lex_order() {
    // No lex option given: the default order applies and registration
    // works.
    let parser = make_mini(one(Block::delimited("tq", "'''", "'''")));
    parse_str(&parser, "'''x'''", "x");
    let matchers = parser.config().lex.matchers;
    let order = matchers
        .get("hoover")
        .expect("the hoover matcher is registered")
        .order;
    assert_eq!(order, DEFAULT_LEX_ORDER);
}

#[test]
fn an_explicit_lex_order_is_honoured() {
    let parser = make_mini(
        HooverOptions::new(vec![Block::delimited("tq", "'''", "'''")]).with_lex_order(1.5e6),
    );
    parse_str(&parser, "'''x'''", "x");
    let matchers = parser.config().lex.matchers;
    assert_eq!(matchers.get("hoover").expect("registered").order, 1.5e6);
}

#[test]
fn does_not_mutate_caller_block_definitions() {
    // The default token is applied to an internal copy, not the caller's
    // definition: the options are cloned into the plugin.
    let block = Block::delimited("tq", "'''", "'''");
    let options = one(block.clone());
    let parser = make_mini(options.clone());
    parse_str(&parser, "'''x'''", "x");
    assert_eq!(options.block[0], block);
    assert_eq!(options.block[0].token, None);
}

#[test]
fn the_token_carries_the_block_name_and_the_action_runs() {
    // The `use` bag of every block token names the block, and the
    // optional action sees the rule whose open token it is.
    let seen: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let options = HooverOptions::new(vec![
        Block::delimited("tq", "'''", "'''"),
        Block::delimited("angle", "<", ">"),
    ])
    .with_action(move |rule, _context| {
        let token = rule.o0().expect("the alt's open token").clone();
        let block = match token.use_data().get("block") {
            Some(Value::String(name)) => name.clone(),
            other => panic!("use.block is not a string: {other:?}"),
        };
        sink.lock()
            .expect("lock")
            .push((token.name.to_string(), block));
        Ok(())
    });
    let parser = make_mini(options);
    parse_str(&parser, "'''a'''", "a");
    parse_str(&parser, "(<b>)", "b");
    let seen = seen.lock().expect("lock").clone();
    assert_eq!(
        seen,
        vec![
            (DEFAULT_TOKEN.to_string(), "tq".to_string()),
            (DEFAULT_TOKEN.to_string(), "angle".to_string()),
        ]
    );
}

#[test]
fn blocks_are_tried_in_order() {
    // Both blocks start with '<'; the first listed wins.
    let parser = make_mini(HooverOptions::new(vec![
        Block::delimited("first", "<", ">"),
        Block::delimited("second", "<", "!"),
    ]));
    parse_str(&parser, "<a>", "a");
    let error = parse_err(&parser, "<a!"); // committed to the first block
    assert_eq!(error.code, "invalid_text");
}

#[test]
fn options_from_json_read_the_shared_data_shape() {
    let document: serde_json::Value = serde_json::from_str(
        r##"{"lex":{"order":1000000},"block":[{"name":"angle","start":{"fixed":["<","~"],"consume":["<"],"rule":{"parent":{"include":["group"]},"state":""}},"end":{"fixed":[">",""],"consume":false},"escapeChar":"\\","escape":{">":">"},"allowUnknownEscape":false,"preserveEscapeChar":true,"trim":true,"token":"#XX"}]}"##,
    )
    .expect("json");
    let options = HooverOptions::from_json(&document).expect("options");
    assert_eq!(options.lex_order, Some(1e6));
    assert_eq!(options.block.len(), 1);
    let block = &options.block[0];
    assert_eq!(block.name, "angle");
    let start = block.start.as_ref().expect("start");
    assert_eq!(start.fixed, Some(vec!["<".to_string(), "~".to_string()]));
    assert_eq!(start.consume, Consume::Only(vec!["<".to_string()]));
    let rule = start.rule.as_ref().expect("rule");
    assert_eq!(rule.parent, Some(HooverRuleFilter::include(&["group"])));
    assert_eq!(rule.current, None);
    assert_eq!(rule.state, Some(String::new()));
    assert_eq!(block.end.fixed, vec![">".to_string(), String::new()]);
    assert_eq!(block.end.consume, Consume::Never);
    assert_eq!(block.escape_char, Some('\\'));
    assert_eq!(block.escape.get(">").map(String::as_str), Some(">"));
    assert_eq!(block.allow_unknown_escape, Some(false));
    assert!(block.preserve_escape_char);
    assert!(block.trim);
    assert_eq!(block.token.as_deref(), Some("#XX"));

    // Go field spellings resolve too, and a JSON null state is unset.
    let document: serde_json::Value = serde_json::from_str(
        r#"{"block":[{"Name":"a","Start":{"Fixed":"~","Rule":{"State":null}},"End":{"Fixed":">"},"EscapeChar":""}]}"#,
    )
    .expect("json");
    let options = HooverOptions::from_json(&document).expect("options");
    let block = &options.block[0];
    assert_eq!(block.name, "a");
    assert_eq!(
        block.start.as_ref().and_then(|s| s.fixed.clone()),
        Some(vec!["~".into()])
    );
    assert_eq!(
        block.start.as_ref().and_then(|s| s.rule.clone()),
        Some(HooverRuleSpec::default())
    );
    assert_eq!(block.escape_char, None);

    // Malformed shapes are errors, not silent defaults.
    for bad in [
        r#"[]"#,
        r#"{"block":{}}"#,
        r#"{"block":[{"name":"a","escapeChar":"ab"}]}"#,
        r#"{"block":[{"name":"a","start":{"fixed":1}}]}"#,
        r#"{"block":[{"name":"a","end":{"consume":"yes"}}]}"#,
        r#"{"lex":{"order":"soon"}}"#,
    ] {
        let document: serde_json::Value = serde_json::from_str(bad).expect("json");
        assert!(
            HooverOptions::from_json(&document).is_err(),
            "{bad} should be refused"
        );
    }
}

// A host grammar that narrows itself with `rule.include`, the shape
// `@tabnas/json` uses (`rule: { include: 'json' }`). The engine's alt
// filter keeps only alts carrying one of the include tags, so hoover's
// added alt has to carry them or the plugin loads without error, the
// matcher fires and produces a `#HV` token, and the parse dies with
// `unexpected` pointing at the user's source rather than at the cause.

fn tagged_blocks() -> HooverOptions {
    one(Block::delimited("triplequote", "'''", "'''"))
}

#[test]
fn hoover_extends_a_grammar_that_sets_rule_include() {
    let parser = make_tagged(tagged_blocks());
    parse_str(&parser, "'''x'''", "x");
    parse_str(&parser, "'''hello world'''", "hello world");
    parse_str(&parser, "'''a\nb'''", "a\nb");
    parse_str(&parser, "('''x''')", "x");
}

#[test]
fn the_host_grammar_still_parses_its_own_values() {
    let parser = make_tagged(tagged_blocks());
    parse_eq(&parser, "true", Value::Bool(true));
    parse_eq(&parser, "123", Value::Number(123.0));
    parse_eq(&parser, "(456)", Value::Number(456.0));
}

#[test]
fn the_block_alternate_survives_the_filter() {
    let parser = make_tagged(tagged_blocks());
    let hv = parser
        .config()
        .token(DEFAULT_TOKEN)
        .expect("#HV is registered");
    let specs = parser.rule_specs();
    let val = specs
        .iter()
        .find(|spec| spec.name == "val")
        .expect("val rule");
    let alt = val
        .open
        .iter()
        .find(|alt| alt.s.iter().any(|position| position.contains(&hv)))
        .expect("the hoover alt is on the val rule");
    assert_eq!(alt.g, "mini", "the alt carries the host's include tag");
}

#[test]
fn hoover_refuses_a_host_whose_filter_excludes_the_block_alternate() {
    // The alt carries the host's include tags, so the only way the
    // filter removes it is a host that excludes one of those same tags.
    // TypeScript throws here; the Rust port returns the error.
    let mut parser = Tabnas::new();
    tag_grammar(&mut parser);
    parser
        .set_options(|options| options.rule.exclude = "mini".to_string())
        .expect("options apply");
    let error = hoover(&mut parser, tagged_blocks())
        .expect_err("hoover must refuse a host whose alt filter drops its alternate");
    assert!(
        error.0.contains("removed from the 'val' rule"),
        "unexpected message: {error}"
    );
    assert!(error.0.contains("rule.include=[\"mini\"]"), "{error}");
    assert!(error.0.contains("rule.exclude=[\"mini\"]"), "{error}");
}

#[test]
fn plugin_form_installs_through_use_plugin() {
    let mut parser = Tabnas::new();
    mini_grammar(&mut parser);
    parser
        .use_plugin(plugin(one(Block::delimited("tq", "'''", "'''"))), None)
        .expect("the plugin installs");
    parse_str(&parser, "'''x'''", "x");
    assert!(parser
        .installed_plugins()
        .iter()
        .any(|installed| installed.name == "Hoover"));
}

#[test]
fn hoover_error_converts_to_and_from_plugin_error() {
    let error = HooverError("boom".into());
    let plugin_error: tabnas::PluginError = error.clone().into();
    assert_eq!(plugin_error.0, "boom");
    let back: HooverError = plugin_error.into();
    assert_eq!(back, error);
    assert_eq!(back.to_string(), "boom");
}

#[test]
fn a_shared_instance_takes_concurrent_callers() {
    // `Tabnas::parse` takes `&self`, so one configured instance serves
    // every thread; the plugin's matcher and action closures are
    // `Send + Sync` by construction.
    let parser = Arc::new(make_mini(one(Block::delimited("tq", "'''", "'''"))));
    let threads: Vec<_> = (0..8)
        .map(|index| {
            let parser = Arc::clone(&parser);
            std::thread::spawn(move || {
                for round in 0..50 {
                    let source = format!("'''t{index} r{round}'''");
                    let want = format!("t{index} r{round}");
                    parse_str(&parser, &source, &want);
                    assert!(parser.parse("'''open").is_err());
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("no thread panicked");
    }
}

#[test]
fn version_is_exported() {
    assert_eq!(VERSION.split('.').count(), 3);
}

#[test]
fn rule_context_parent_at_the_start_rule() {
    // The start rule has no parent. In TypeScript it has an unnamed
    // sentinel one, so no `parent.include` entry matches it and every
    // `parent.exclude` passes; nothing a user can list, "none" included,
    // names it.
    let parser = make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            parent: Some(HooverRuleFilter::include(&["none"])),
            ..Default::default()
        },
    )));
    parse_str(&parser, "@hi@", "@hi@"); // include never matches at the root
    let parser = make_mini(one(Block::delimited("at", "@", "@").with_rule(
        HooverRuleSpec {
            parent: Some(HooverRuleFilter::exclude(&["none"])),
            ..Default::default()
        },
    )));
    parse_str(&parser, "@hi@", "hi"); // exclude always passes at the root
}

#[test]
fn row_after_a_mapped_escape_follows_the_source() {
    // Recorded in ../DIVERGENCE.md ("Row after a mapped escape").
    // TypeScript and Go advance the row when a mapped escape's REPLACEMENT
    // is a newline; this port advances the engine's cursor over the
    // consumed SOURCE, and the engine offers no way to do otherwise. So
    // the stray `%` is reported on the row the source has. If these
    // assertions start failing with the TypeScript rows (2:4, then 1:8),
    // delete the DIVERGENCE.md entry along with this test.
    let parser = make_mini(one(Block::delimited("angle", "<", ">")
        .with_escape_char('\\')
        .with_escape("n", "\n")));
    parse_str(&parser, "<a\\nb>", "a\nb"); // the value agrees everywhere
    let error = parse_err(&parser, "<a\\nb> %");
    assert_eq!(
        (error.code.as_str(), error.row, error.col),
        ("unexpected", 1, 8)
    );

    let parser = make_mini(one(Block::delimited("angle", "<", ">")
        .with_escape_char('\\')
        .with_escape("\n", "N")));
    parse_str(&parser, "<a\\\nb>", "aNb");
    let error = parse_err(&parser, "<a\\\nb> %");
    assert_eq!(
        (error.code.as_str(), error.row, error.col),
        ("unexpected", 2, 4)
    );
}

#[test]
fn an_astral_escape_key_never_matches_as_in_typescript() {
    // TypeScript indexes one UTF-16 code unit after the escape character,
    // so a key outside the BMP cannot match and the character takes the
    // unknown-escape path. The Rust lookup could see the whole character
    // and must not.
    let parser = make_mini(one(Block::delimited("angle", "<", ">")
        .with_escape_char('\\')
        .with_escape("\u{1F600}", "X")
        .with_escape("n", "\n")));
    parse_str(&parser, "<a\\\u{1F600}b>", "a\u{1F600}b"); // unknown escape: char kept
    parse_str(&parser, "<a\\nb>", "a\nb"); // a BMP key still maps
}

#[test]
fn a_malformed_start_or_end_is_rejected_by_from_json() {
    let bad_start: serde_json::Value =
        serde_json::from_str(r#"{"block":[{"name":"angle","start":"<","end":{"fixed":">"}}]}"#)
            .expect("json");
    let error = HooverOptions::from_json(&bad_start).expect_err("start must be an object");
    assert!(
        error.to_string().contains(".start must be an object"),
        "{error}"
    );

    let bad_end: serde_json::Value =
        serde_json::from_str(r#"{"block":[{"name":"angle","start":{"fixed":"<"},"end":5}]}"#)
            .expect("json");
    let error = HooverOptions::from_json(&bad_end).expect_err("end must be an object");
    assert!(
        error.to_string().contains(".end must be an object"),
        "{error}"
    );

    // null is "absent", as in both other runtimes.
    let null_start: serde_json::Value =
        serde_json::from_str(r#"{"block":[{"name":"angle","start":null,"end":{"fixed":">"}}]}"#)
            .expect("json");
    let options = HooverOptions::from_json(&null_start).expect("null start is absent");
    assert!(options.block[0].start.is_none());
}

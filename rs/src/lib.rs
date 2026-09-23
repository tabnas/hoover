/* Copyright (c) 2021-2026 Richard Rodger, MIT License */

//! Block-delimited string *hoovering* for the
//! [tabnas](https://github.com/tabnas/parser) parser engine.
//!
//! Hoovering vacuums up a run of source text, internal spaces and
//! newlines included, between a start and an end delimiter, with
//! optional escape handling and rule-context gating. The two canonical
//! shapes are triple-quoted strings (`'''hello world'''`) and
//! end-of-line values (an unquoted run of text up to a newline, `#`, `;`
//! or the end of the input).
//!
//! This crate is **not a grammar**: it is a lexer-matcher plugin that
//! extends the host grammar's `val` rule, so it is applied to a
//! [`Tabnas`] instance that already carries a grammar defining `val`. It
//! installs one custom lex matcher (`lex.match.hoover`, at order `4.5e6`,
//! ahead of the string and number matchers) that emits a `#HV` token, and
//! prepends a `val` alternate accepting that token.
//!
//! ```no_run
//! use tabnas::Tabnas;
//! use tabnas_hoover::{hoover, Block, HooverOptions};
//!
//! let mut parser = Tabnas::new();
//! // ... install a host grammar that defines the `val` rule ...
//! hoover(
//!     &mut parser,
//!     HooverOptions::new(vec![Block::delimited("triplequote", "'''", "'''")]),
//! )?;
//! let value = parser.parse("'''hello world'''")?; // "hello world"
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! TypeScript is canonical: `ts/src/hoover.ts` defines behaviour, option
//! names and defaults, and `go/hoover.go` is the sibling port. The shared
//! fixtures in `test/spec/*.tsv` are the parity contract across all
//! three.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[cfg(feature = "serde_json")]
use serde_json::Value as JsonValue;
use tabnas::{
    ActionError, Context, GrammarError, GrammarSpec, ImperativeLexMatcher, Lexer, Options, Plugin,
    PluginError, Rule, RuleState, Tabnas, Tin, Token, Value,
};

/// The README's Rust examples run as doctests, so a stale one fails the
/// gate rather than misleading the reader. Its `text`, `toml` and `bash`
/// fences are skipped; rustdoc runs only the `rust` ones.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}

/// VERSION is this crate's version. It MUST equal `ts/package.json`
/// "version" and the `version` field in `rs/Cargo.toml`: the release
/// orchestrator rewrites all of them, and `tests/version_test.rs` fails
/// the build if they drift. Mirrors `VERSION` in `ts/src/hoover.ts` and
/// `const VERSION` in `go/hoover.go`.
pub const VERSION: &str = "0.3.7";

/// The default matcher order: before the string (`5e6`) and number
/// (`7e6`) matchers. Mirrors `Hoover.defaults.lex.order` in TypeScript
/// and `Defaults["lex"]["order"]` in Go.
pub const DEFAULT_LEX_ORDER: f64 = 4.5e6;

/// The default token name a block emits.
pub const DEFAULT_TOKEN: &str = "#HV";

/// The error code a committed block that never reaches an end
/// delimiter is rejected with.
pub const INVALID_TEXT: &str = "invalid_text";

/// The error code a rejected escape (`allow_unknown_escape: false` and an
/// escape not in the `escape` map) is rejected with.
pub const INVALID_ESCAPE: &str = "invalid_escape";

/// A registration failure: the host grammar defines no `val` rule, the
/// host's alt filter removed the block alternate, the options could not
/// be read, or the engine refused the grammar.
///
/// The TypeScript plugin *throws* for these; the Rust port returns them,
/// exactly as the Go port does. The plugin itself never panics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HooverError(pub String);

impl fmt::Display for HooverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for HooverError {}

impl From<PluginError> for HooverError {
    fn from(error: PluginError) -> Self {
        Self(error.0)
    }
}

impl From<GrammarError> for HooverError {
    fn from(error: GrammarError) -> Self {
        Self(error.0)
    }
}

impl From<HooverError> for PluginError {
    fn from(error: HooverError) -> Self {
        Self(error.0)
    }
}

/// The alternate action run when a block token is matched by the `val`
/// rule, the Rust spelling of the TypeScript `action?: AltAction`
/// option. It sees the rule whose open token is the block token (its
/// `use` bag carries `{block: <name>}`) and the parse context.
pub type ActionFn = Arc<dyn Fn(&mut Rule, &mut Context) -> Result<(), ActionError> + Send + Sync>;

/// Include and exclude lists for one side of a rule-context filter.
///
/// An absent list imposes no constraint; an empty list is a constraint
/// that nothing satisfies (`include`) or everything satisfies
/// (`exclude`), exactly as in TypeScript.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HooverRuleFilter {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

impl HooverRuleFilter {
    /// A filter that passes only the named rules.
    pub fn include(names: &[&str]) -> Self {
        Self {
            include: Some(names.iter().map(|name| (*name).to_string()).collect()),
            exclude: None,
        }
    }

    /// A filter that rejects the named rules.
    pub fn exclude(names: &[&str]) -> Self {
        Self {
            include: None,
            exclude: Some(names.iter().map(|name| (*name).to_string()).collect()),
        }
    }

    /// `name` is the empty string for the start rule's parent: TypeScript's
    /// sentinel rule there is named `""`, so `""` is the one entry that
    /// lists it.
    fn passes(&self, name: &str, match_rule: &mut Option<bool>) {
        let listed = |list: &[String]| list.iter().any(|entry| entry == name);
        if let Some(include) = &self.include {
            and_into(match_rule, listed(include));
        }
        if let Some(exclude) = &self.exclude {
            and_into(match_rule, !listed(exclude));
        }
    }
}

/// The rule context a block start is gated on: filters on the current
/// rule and its parent, plus the rule state.
///
/// `state` selects which rule states match: `"o"`, `"c"` or `"oc"`
/// check the state; `None` defaults to `"o"` (open); `Some("")` means
/// *do not check the state at all* (the TypeScript `state: ''` form,
/// which Go spells `StateAny`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HooverRuleSpec {
    pub parent: Option<HooverRuleFilter>,
    pub current: Option<HooverRuleFilter>,
    pub state: Option<String>,
}

/// Whether a matched delimiter is consumed (removed from the value and
/// the stream) or left in place. Mirrors the TypeScript
/// `consume?: null | boolean | string[]` field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Consume {
    /// Consume every matched delimiter (the default; `null` or `true`).
    #[default]
    All,
    /// Consume nothing (`false`): the delimiter stays in the stream, and a
    /// start delimiter becomes part of the value.
    Never,
    /// Consume only the listed delimiters.
    Only(Vec<String>),
}

impl Consume {
    fn applies_to(&self, delimiter: &str) -> bool {
        match self {
            Consume::All => true,
            Consume::Never => false,
            Consume::Only(list) => list.iter().any(|entry| entry == delimiter),
        }
    }
}

/// How a block starts: optional fixed delimiters, whether a matched one
/// is consumed, and the rule context the start is gated on.
///
/// With no `fixed` delimiters the block starts wherever its rule context
/// matches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartSpec {
    pub fixed: Option<Vec<String>>,
    pub consume: Consume,
    pub rule: Option<HooverRuleSpec>,
}

/// How a block ends: the delimiters it scans for, and whether a matched
/// one is consumed. The empty string `""` names the end of the input.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndSpec {
    pub fixed: Vec<String>,
    pub consume: Consume,
}

/// One block definition. Blocks are tried in the order they are listed
/// in [`HooverOptions::block`], so that order is significant.
///
/// Once a block's start matches, the block is *committed*: failing to
/// reach an end delimiter is an `invalid_text` error and a rejected
/// escape is an `invalid_escape` error, never a fall-through to the next
/// block or matcher.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Block {
    /// The block name, carried on every token it emits as `use.block`.
    pub name: String,
    pub start: Option<StartSpec>,
    pub end: EndSpec,
    /// The token name the block emits; `None` means [`DEFAULT_TOKEN`].
    pub token: Option<String>,
    /// The escape character, when the block has one.
    pub escape_char: Option<char>,
    /// Replacements for escaped characters, keyed by the character
    /// following the escape character.
    pub escape: HashMap<String, String>,
    /// Whether an escape absent from `escape` is allowed (the escape
    /// character is dropped, or kept with `preserve_escape_char`);
    /// `None` means `true`.
    pub allow_unknown_escape: Option<bool>,
    /// Keep the escape character in the value for an unknown escape.
    pub preserve_escape_char: bool,
    /// Trim the hoovered value with the JavaScript `String.prototype.trim`
    /// character set.
    pub trim: bool,
}

impl Block {
    /// A block named `name` with no delimiters; fill the fields in.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// A block bounded by one fixed start and one fixed end delimiter,
    /// both consumed.
    pub fn delimited(name: impl Into<String>, start: &str, end: &str) -> Self {
        Self::new(name).with_start(&[start]).with_end(&[end])
    }

    /// Set the fixed start delimiters, keeping any rule context.
    pub fn with_start(mut self, fixed: &[&str]) -> Self {
        let start = self.start.get_or_insert_with(StartSpec::default);
        start.fixed = Some(fixed.iter().map(|entry| (*entry).to_string()).collect());
        self
    }

    /// Set the fixed end delimiters (`""` is the end of the input).
    pub fn with_end(mut self, fixed: &[&str]) -> Self {
        self.end.fixed = fixed.iter().map(|entry| (*entry).to_string()).collect();
        self
    }

    /// Gate the block start on a rule context.
    pub fn with_rule(mut self, rule: HooverRuleSpec) -> Self {
        self.start.get_or_insert_with(StartSpec::default).rule = Some(rule);
        self
    }

    /// Set the escape character.
    pub fn with_escape_char(mut self, escape_char: char) -> Self {
        self.escape_char = Some(escape_char);
        self
    }

    /// Add an escape replacement.
    pub fn with_escape(mut self, escaped: &str, replacement: &str) -> Self {
        self.escape
            .insert(escaped.to_string(), replacement.to_string());
        self
    }

    fn token_name(&self) -> &str {
        self.token.as_deref().unwrap_or(DEFAULT_TOKEN)
    }

    fn allow_unknown(&self) -> bool {
        self.allow_unknown_escape.unwrap_or(true)
    }
}

/// The plugin options: the ordered block list, the matcher order and
/// the optional alternate action.
///
/// Typed rather than a serialized bag because the action is a Rust
/// callback and no [`Value`] can carry one. The data shape the other
/// runtimes accept (a JSON object with `block`, `lex.order`) is read by
/// [`HooverOptions::from_json`].
#[derive(Clone, Default)]
pub struct HooverOptions {
    /// The blocks, tried in order.
    pub block: Vec<Block>,
    /// The custom matcher's order; `None` means [`DEFAULT_LEX_ORDER`].
    pub lex_order: Option<f64>,
    /// The alternate action run when a block token is matched.
    pub action: Option<ActionFn>,
}

impl fmt::Debug for HooverOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HooverOptions")
            .field("block", &self.block)
            .field("lex_order", &self.lex_order)
            .field("action", &self.action.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl HooverOptions {
    /// Options over the given blocks, with the default matcher order and
    /// no action.
    pub fn new(block: Vec<Block>) -> Self {
        Self {
            block,
            lex_order: None,
            action: None,
        }
    }

    /// Set the custom matcher's order.
    pub fn with_lex_order(mut self, order: f64) -> Self {
        self.lex_order = Some(order);
        self
    }

    /// Set the alternate action.
    pub fn with_action(
        mut self,
        action: impl Fn(&mut Rule, &mut Context) -> Result<(), ActionError> + Send + Sync + 'static,
    ) -> Self {
        self.action = Some(Arc::new(action));
        self
    }

    /// Read the data shape the TypeScript plugin takes as an object
    /// literal and the Go plugin takes as a map: `{"block": [...],
    /// "lex": {"order": n}}`, with each block's fields spelled as in
    /// TypeScript (`escapeChar`, `allowUnknownEscape`, ...). Keys are
    /// matched case-insensitively, as in Go, so the Go field spellings
    /// resolve too. `fixed` may be a string or a list, `consume` a
    /// boolean or a list, and `"state": ""` means "do not check the
    /// state".
    ///
    /// An `escapeChar` longer than one character, or a value of the
    /// wrong type where the shape is fixed, is an error. Unknown keys are
    /// ignored, as they are in both other runtimes.
    ///
    /// Available with the `serde_json` feature, which is off by default
    /// so that the crate's only production dependency stays the engine.
    #[cfg(feature = "serde_json")]
    pub fn from_json(document: &JsonValue) -> Result<Self, HooverError> {
        let map = document
            .as_object()
            .ok_or_else(|| HooverError("hoover: options must be an object".into()))?;
        let mut options = Self::default();

        if let Some(blocks) = field(map, "block") {
            let blocks = blocks.as_array().ok_or_else(|| {
                HooverError("hoover: options.block must be an array of blocks".into())
            })?;
            options.block = blocks
                .iter()
                .map(block_from_json)
                .collect::<Result<Vec<_>, _>>()?;
        }
        if let Some(lex) = field(map, "lex").and_then(JsonValue::as_object) {
            if let Some(order) = field(lex, "order") {
                options.lex_order = Some(order.as_f64().ok_or_else(|| {
                    HooverError("hoover: options.lex.order must be a number".into())
                })?);
            }
        }
        Ok(options)
    }
}

/// Look a key up exactly, then case-insensitively, so both the TypeScript
/// spelling (`escapeChar`) and the Go field name (`EscapeChar`) resolve.
#[cfg(feature = "serde_json")]
fn field<'a>(map: &'a serde_json::Map<String, JsonValue>, key: &str) -> Option<&'a JsonValue> {
    map.get(key).or_else(|| {
        map.iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
            .map(|(_, value)| value)
    })
}

/// A `fixed`-style field: a single string or a list of them.
#[cfg(feature = "serde_json")]
fn strings_from_json(value: &JsonValue, label: &str) -> Result<Vec<String>, HooverError> {
    match value {
        JsonValue::String(text) => Ok(vec![text.clone()]),
        JsonValue::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| HooverError(format!("hoover: {label} entries must be strings")))
            })
            .collect(),
        _ => Err(HooverError(format!(
            "hoover: {label} must be a string or a list of strings"
        ))),
    }
}

#[cfg(feature = "serde_json")]
fn consume_from_json(
    map: &serde_json::Map<String, JsonValue>,
    label: &str,
) -> Result<Consume, HooverError> {
    match field(map, "consume") {
        None | Some(JsonValue::Null) | Some(JsonValue::Bool(true)) => Ok(Consume::All),
        Some(JsonValue::Bool(false)) => Ok(Consume::Never),
        Some(list @ JsonValue::Array(_)) => Ok(Consume::Only(strings_from_json(
            list,
            &format!("{label}.consume"),
        )?)),
        Some(_) => Err(HooverError(format!(
            "hoover: {label}.consume must be a boolean or a list of strings"
        ))),
    }
}

#[cfg(feature = "serde_json")]
fn filter_from_json(value: &JsonValue, label: &str) -> Result<HooverRuleFilter, HooverError> {
    let map = value
        .as_object()
        .ok_or_else(|| HooverError(format!("hoover: {label} must be an object")))?;
    Ok(HooverRuleFilter {
        include: field(map, "include")
            .map(|list| strings_from_json(list, &format!("{label}.include")))
            .transpose()?,
        exclude: field(map, "exclude")
            .map(|list| strings_from_json(list, &format!("{label}.exclude")))
            .transpose()?,
    })
}

#[cfg(feature = "serde_json")]
fn rule_from_json(value: &JsonValue, label: &str) -> Result<HooverRuleSpec, HooverError> {
    let map = value
        .as_object()
        .ok_or_else(|| HooverError(format!("hoover: {label} must be an object")))?;
    Ok(HooverRuleSpec {
        parent: field(map, "parent")
            .map(|filter| filter_from_json(filter, &format!("{label}.parent")))
            .transpose()?,
        current: field(map, "current")
            .map(|filter| filter_from_json(filter, &format!("{label}.current")))
            .transpose()?,
        // A JSON null is NOT the empty string: it means the key carries no
        // value, so the "o" default applies. Only a string is a setting.
        state: field(map, "state")
            .and_then(JsonValue::as_str)
            .map(str::to_string),
    })
}

#[cfg(feature = "serde_json")]
fn block_from_json(value: &JsonValue) -> Result<Block, HooverError> {
    let map = value
        .as_object()
        .ok_or_else(|| HooverError("hoover: each block must be an object".into()))?;
    let name = field(map, "name")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();
    let label = format!("block {name:?}");
    let mut block = Block::new(name);

    if let Some(token) = field(map, "token").and_then(JsonValue::as_str) {
        block.token = Some(token.to_string());
    }
    if let Some(escape_char) = field(map, "escapeChar").and_then(JsonValue::as_str) {
        let mut chars = escape_char.chars();
        block.escape_char = match (chars.next(), chars.next()) {
            (None, _) => None,
            (Some(first), None) => Some(first),
            (Some(_), Some(_)) => {
                return Err(HooverError(format!(
                    "hoover: {label}.escapeChar must be a single character"
                )))
            }
        };
    }
    if let Some(escape) = field(map, "escape").and_then(JsonValue::as_object) {
        for (key, replacement) in escape {
            if let Some(replacement) = replacement.as_str() {
                block.escape.insert(key.clone(), replacement.to_string());
            }
        }
    }
    if let Some(allow) = field(map, "allowUnknownEscape").and_then(JsonValue::as_bool) {
        block.allow_unknown_escape = Some(allow);
    }
    block.preserve_escape_char = field(map, "preserveEscapeChar")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    block.trim = field(map, "trim")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);

    // A present `start` or `end` of the wrong shape is a configuration
    // error, not an absent one: silently dropping it would broaden the
    // match to "no fixed delimiter" and hoover unrelated input.
    if let Some(start) = field(map, "start") {
        if !start.is_null() && !start.is_object() {
            return Err(HooverError(format!(
                "hoover: {label}.start must be an object"
            )));
        }
    }
    if let Some(end) = field(map, "end") {
        if !end.is_null() && !end.is_object() {
            return Err(HooverError(format!(
                "hoover: {label}.end must be an object"
            )));
        }
    }
    if let Some(start) = field(map, "start").and_then(JsonValue::as_object) {
        let start_label = format!("{label}.start");
        block.start = Some(StartSpec {
            fixed: field(start, "fixed")
                .map(|fixed| strings_from_json(fixed, &format!("{start_label}.fixed")))
                .transpose()?,
            consume: consume_from_json(start, &start_label)?,
            rule: field(start, "rule")
                .map(|rule| rule_from_json(rule, &format!("{start_label}.rule")))
                .transpose()?,
        });
    }
    if let Some(end) = field(map, "end").and_then(JsonValue::as_object) {
        let end_label = format!("{label}.end");
        block.end = EndSpec {
            fixed: field(end, "fixed")
                .map(|fixed| strings_from_json(fixed, &format!("{end_label}.fixed")))
                .transpose()?
                .unwrap_or_default(),
            consume: consume_from_json(end, &end_label)?,
        };
    }
    Ok(block)
}

/// Build the hoover plugin for `options`.
///
/// The returned [`Plugin`] can be installed with [`Tabnas::use_plugin`]
/// and, like every native plugin, re-runs against a derived instance.
/// Most callers want [`hoover`].
pub fn plugin(options: HooverOptions) -> Plugin {
    Plugin::new("Hoover", move |parser, _plugin_options| {
        install(parser, &options).map_err(PluginError::from)
    })
}

/// Register hoover on `parser`, the Rust spelling of the TypeScript
/// `tn.use(Hoover, options)` call.
///
/// The parser must already carry a grammar that defines the `val` rule:
/// hoover extends that rule, and fails fast with a clear error when it is
/// absent rather than creating an empty one and failing confusingly
/// later. It also refuses when the host grammar's alt filter
/// (`rule.include` / `rule.exclude`) would discard the block alternate.
pub fn hoover(parser: &mut Tabnas, options: HooverOptions) -> Result<(), HooverError> {
    parser
        .use_plugin(plugin(options), None)
        .map(|_| ())
        .map_err(HooverError::from)
}

/// One block as the matcher sees it: the definition plus the token
/// identity it emits.
struct Prepared {
    block: Block,
    token_name: String,
    tin: Tin,
}

/// The slice of the resolved lexer configuration `parse_to_end` reads:
/// the TypeScript `cfg.value.lex` flag and `cfg.value.def` table.
struct MatcherConfig {
    value_lex: bool,
    definitions: HashMap<String, Value>,
}

impl MatcherConfig {
    fn from_options(options: &Options) -> Self {
        Self {
            value_lex: options.value.lex,
            definitions: options
                .value
                .definitions
                .iter()
                .map(|(source, definition)| {
                    (
                        source.clone(),
                        definition.val.clone().unwrap_or(Value::Undefined),
                    )
                })
                .collect(),
        }
    }
}

/// Each install registers its own references, so two hoover
/// registrations on one instance (or on a derived one) do not overwrite
/// each other's action or matcher factory.
static INSTALLS: AtomicUsize = AtomicUsize::new(0);

/// Quote `text` as a JSON string literal for the grammar document.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn install(parser: &mut Tabnas, options: &HooverOptions) -> Result<(), HooverError> {
    // Hoover extends the host grammar's `val` rule. Fail fast with a clear
    // message if a grammar providing it has not been registered first.
    if !parser.rule_names().iter().any(|name| name == "val") {
        return Err(HooverError(
            "tabnas-hoover: the 'val' rule is missing; \
             register a grammar that defines it before the hoover plugin"
                .into(),
        ));
    }

    let id = INSTALLS.fetch_add(1, Ordering::Relaxed);
    let make_ref = format!("@hoover-matcher-{id}");
    let action_ref = format!("@hoover-action-{id}");

    // The engine's `rule.include` filter keeps only those alts carrying
    // one of its group tags, so an untagged alt added to a host that
    // narrows itself (`@tabnas/json` uses `rule: { include: 'json' }`)
    // would be silently discarded. Carrying the host's active tags says
    // "this alt is part of the selected grammar", which is exactly what a
    // deliberate registration means.
    let config = parser.config();
    let include: Vec<String> = listed(&config.rule.include).map(str::to_string).collect();
    let groups = include.join(",");

    let mut prepared: Vec<Prepared> = Vec::with_capacity(options.block.len());
    let mut token_map: Vec<(String, Tin)> = Vec::new();
    let mut alts: Vec<String> = Vec::new();

    if let Some(action) = options.action.clone() {
        parser.action_with_context(action_ref.clone(), move |rule, context| {
            action(rule, context)
        });
    }

    for block in &options.block {
        let token_name = block.token_name().to_string();
        // The alt is declared through the serialized document, where a
        // token name is one whitespace-free word; refuse anything else
        // here, with a message that names the cause, rather than let the
        // alt filter check below report it as an exclusion.
        if token_name.is_empty() || token_name.chars().any(char::is_whitespace) {
            return Err(HooverError(format!(
                "tabnas-hoover: block {:?} has the token name {:?}; \
                 a token name is a non-empty word without whitespace",
                block.name, token_name
            )));
        }
        let tin = parser.token(token_name.clone());

        // Only the first occurrence of each token name adds a `val`
        // alternate. Each is PREPENDED, as `rs.open(...)` does in
        // TypeScript, so the alts are listed in reverse block order.
        if !token_map.iter().any(|(name, _)| *name == token_name) {
            let mut alt = format!("{{\"s\":{}", json_string(&token_name));
            if options.action.is_some() {
                alt.push_str(&format!(",\"a\":{}", json_string(&action_ref)));
            }
            if !groups.is_empty() {
                alt.push_str(&format!(",\"g\":{}", json_string(&groups)));
            }
            alt.push('}');
            alts.insert(0, alt);
            token_map.push((token_name.clone(), tin));
        }

        prepared.push(Prepared {
            block: block.clone(),
            token_name,
            tin,
        });
    }

    let blocks = Arc::new(prepared);
    parser.lex_match_factory_ref(make_ref.clone(), move |resolved: &Options| {
        let config = Arc::new(MatcherConfig::from_options(resolved));
        let blocks = Arc::clone(&blocks);
        let matcher: ImperativeLexMatcher = Arc::new(
            move |lexer: &mut Lexer<'_>, rule: &mut Rule, _context: &mut Context| {
                hoover_matcher(lexer, rule, &blocks, &config)
            },
        );
        Some(matcher)
    });

    let order = options.lex_order.unwrap_or(DEFAULT_LEX_ORDER);
    if !order.is_finite() {
        return Err(HooverError(
            "hoover: lex.order must be a finite number".into(),
        ));
    }
    // Written as JSON text rather than built with serde_json, so the
    // engine stays this crate's only production dependency.
    let document = format!(
        "{{\"options\":{{\"lex\":{{\"match\":{{\"hoover\":{{\"order\":{order},\"make\":{make}}}}}}}}},\
         \"rule\":{{\"val\":{{\"open\":[{alts}]}}}}}}",
        make = json_string(&make_ref),
        alts = alts.join(",")
    );
    let spec = GrammarSpec::from_json(&document)?;
    parser.grammar(&spec)?;

    // Check rather than assume that the alts survive the host's alt
    // filter: the failure mode is silent, a working lexer feeding a rule
    // that cannot accept its token, and it surfaces to the user as an
    // `unexpected` error on their own source, pointing nowhere near the
    // cause.
    let config = parser.config();
    let survivors = parser
        .rule_specs()
        .into_iter()
        .find(|spec| spec.name == "val")
        .map(|spec| spec.open.clone())
        .unwrap_or_default();
    let missing: Vec<&str> = token_map
        .iter()
        .filter(|(_, tin)| {
            !survivors.iter().any(|alt| {
                alt.s.iter().any(|position| position.contains(tin))
                    && groups_enabled(&alt.g, &config.rule.include, &config.rule.exclude)
            })
        })
        .map(|(name, _)| name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(HooverError(format!(
            "tabnas-hoover: the block alternate was removed from the 'val' rule by the \
             host grammar's alt filter (rule.include={:?}, rule.exclude={:?}); \
             hoover cannot extend a grammar that excludes it",
            include,
            listed(&config.rule.exclude).collect::<Vec<_>>(),
        )));
    }

    Ok(())
}

/// Split a comma-separated group list, dropping empty entries: the
/// engine's own reading of `rule.include` and `rule.exclude`.
fn listed(list: &str) -> impl Iterator<Item = &str> {
    list.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
}

/// The engine's alt filter, as `parser.rs` applies it: with neither list
/// set every alt is enabled; otherwise an alt must carry one of the
/// included groups (when any are listed) and none of the excluded ones.
fn groups_enabled(alt_groups: &str, include: &str, exclude: &str) -> bool {
    if include.is_empty() && exclude.is_empty() {
        return true;
    }
    let declares = |wanted: &str| listed(alt_groups).any(|group| group == wanted);
    let included = listed(include).next().is_none() || listed(include).any(declares);
    included && !listed(exclude).any(declares)
}

/// `matchRule = value && (null === matchRule ? true : matchRule)`: each
/// evaluated condition ANDs itself into the tri-state result.
fn and_into(match_rule: &mut Option<bool>, value: bool) {
    *match_rule = Some(value && match_rule.unwrap_or(true));
}

/// The lex matcher: try each block in order; the first whose start
/// matches is committed.
fn hoover_matcher(
    lexer: &mut Lexer<'_>,
    rule: &mut Rule,
    blocks: &[Prepared],
    config: &MatcherConfig,
) -> Option<Token> {
    for prepared in blocks {
        let block = &prepared.block;
        let point = lexer.point();
        let rest = lexer.remaining();

        let Some(start) = match_start(rest, rule, block) else {
            continue;
        };

        match parse_to_end(rest, start, block, config) {
            Ok(Scanned { value, end }) => {
                let source = &rest[..end];
                let mut token =
                    lexer.token(&prepared.token_name, prepared.tin, value, source, point);
                token
                    .use_data_mut()
                    .insert("block".to_string(), Value::String(block.name.clone()));
                let consumed = source.chars().count();
                lexer.advance_chars(consumed);
                return Some(token);
            }
            // A rejected escape: the bad token spans the escape character.
            Err(Rejected::Escape { at }) => {
                let position = point.site.pos + rest[..at].chars().count();
                return Some(lexer.bad_span(INVALID_ESCAPE, position, position + 1));
            }
            // Once a start matches, the block is committed: never reaching
            // an end delimiter is an error, not a fall-through. The span
            // is the start delimiter, as in TypeScript.
            Err(Rejected::Unterminated) => {
                let position = point.site.pos;
                let width = rest[..start].chars().count();
                return Some(lexer.bad_span(INVALID_TEXT, position, position + width));
            }
        }
    }
    None
}

/// Match a block's start against the rule context and the source at the
/// cursor. On a match, the byte length of the consumed start delimiter
/// (zero when it is kept or there is none).
fn match_start(rest: &str, rule: &Rule, block: &Block) -> Option<usize> {
    let start = block.start.as_ref();
    let rulespec = start.and_then(|start| start.rule.as_ref());

    // `None` means "no rule condition has been evaluated", which is not
    // the same as "a condition failed": with no conditions there is
    // nothing to constrain the match, so it passes.
    let mut match_rule: Option<bool> = None;

    // NOTE: the default rule state is open ('o'); there is no default
    // parent or current filter, an absent filter imposes no constraint.
    if let Some(rulespec) = rulespec {
        if let Some(parent) = &rulespec.parent {
            // The start rule has no parent here; in the canonical engine
            // it has a sentinel one whose name is the empty string, so
            // `""` is the one entry a list can name it by. Read the
            // missing parent as that name so the two agree.
            let parent_name = rule
                .parent_rule
                .as_deref()
                .map_or("", |parent| parent.name.as_ref());
            parent.passes(parent_name, &mut match_rule);
        }
        if let Some(current) = &rulespec.current {
            current.passes(rule.name.as_ref(), &mut match_rule);
        }
    }

    // '': don't check; 'oc' | 'c' | 'o' check; default 'o'.
    let rulestate = rulespec
        .and_then(|spec| spec.state.as_deref())
        .unwrap_or("o");
    if !rulestate.is_empty() {
        let state = match rule.state {
            RuleState::Open => 'o',
            RuleState::Close => 'c',
        };
        and_into(&mut match_rule, rulestate.contains(state));
    }

    // Resolve "no condition evaluated" to a pass.
    if !match_rule.unwrap_or(true) {
        return None;
    }

    let Some(fixed) = start.and_then(|start| start.fixed.as_ref()) else {
        return Some(0);
    };
    let consume = start.map(|start| &start.consume);
    for delimiter in fixed {
        if rest.starts_with(delimiter.as_str()) {
            let consumed = consume.is_none_or(|consume| consume.applies_to(delimiter));
            return Some(if consumed { delimiter.len() } else { 0 });
        }
    }
    None
}

/// The result of a completed scan: the value, and the byte offset in the
/// remaining source just past everything the block consumed.
struct Scanned {
    value: Value,
    end: usize,
}

enum Rejected {
    /// An escape not in the map, with unknown escapes disallowed; `at` is
    /// the byte offset of the escape character.
    Escape { at: usize },
    /// No end delimiter was reached.
    Unterminated,
}

/// Scan forward from `start` for one of the block's end delimiters,
/// applying escapes, then trim and resolve the value.
fn parse_to_end(
    rest: &str,
    start: usize,
    block: &Block,
    config: &MatcherConfig,
) -> Result<Scanned, Rejected> {
    let fixed = &block.end.fixed;

    // The first character of each end delimiter and the tail that must
    // follow it. The `""` (end-of-input) delimiter has no first
    // character, which nothing in the source equals, so `None` is the
    // out-of-band marker TypeScript gets from `undefined`.
    let endchars: Vec<Option<char>> = fixed.iter().map(|end| end.chars().next()).collect();
    let endseqs: Vec<&str> = fixed
        .iter()
        .map(|end| end.chars().next().map_or("", |c| &end[c.len_utf8()..]))
        .collect();

    let mut value = String::new();
    let mut index = start;
    let mut done = false;
    let mut end_index = index;

    loop {
        let current = rest[index..].chars().next();

        // Check for an end delimiter first, so an escape character that
        // is also an end character ends the block.
        if let Some(found) = endchars.iter().position(|end| *end == current) {
            let tail = endseqs[found];
            let after = index + current.map_or(0, char::len_utf8);
            if current.is_none() || tail.is_empty() {
                end_index = after;
                done = true;
                break;
            }
            if rest[after..].starts_with(tail) {
                end_index = after + tail.len();
                done = true;
                break;
            }
        }

        let Some(c) = current else {
            break;
        };

        if Some(c) == block.escape_char {
            let next = index + c.len_utf8();
            // An escape char as the final source character has nothing to
            // escape: the block never reaches an end delimiter and is
            // reported as unterminated.
            let Some(escaped) = rest[next..].chars().next() else {
                return Err(Rejected::Unterminated);
            };
            let after = next + escaped.len_utf8();
            // TypeScript reads one UTF-16 code unit here, so a character
            // outside the BMP can never match a key of the escape map and
            // always takes the unknown-escape path. Same here.
            let mapped = if (escaped as u32) <= 0xFFFF {
                block.escape.get(&escaped.to_string())
            } else {
                None
            };
            if let Some(replacement) = mapped {
                value.push_str(replacement);
            } else if block.allow_unknown() {
                if block.preserve_escape_char {
                    value.push(c);
                }
                value.push(escaped);
            } else {
                return Err(Rejected::Escape { at: index });
            }
            index = after;
        } else {
            value.push(c);
            index += c.len_utf8();
        }
    }

    if !done {
        return Err(Rejected::Unterminated);
    }

    let endfixed = &rest[index..end_index];
    if block.end.consume.applies_to(endfixed) {
        index = end_index;
    }

    let text = if block.trim {
        js_trim(&value).to_string()
    } else {
        value
    };

    let value = match config.definitions.get(&text) {
        Some(defined) if config.value_lex => defined.clone(),
        _ => Value::String(text),
    };

    Ok(Scanned { value, end: index })
}

/// Whether `c` is removed by JavaScript's `String.prototype.trim`, which
/// the canonical port calls for `trim: true`.
///
/// ECMA-262 defines the trimmed set as WhiteSpace plus LineTerminator:
/// TAB, VT, FF, ZWNBSP (U+FEFF), every Space_Separator (Zs) code point,
/// LF, CR, LS (U+2028) and PS (U+2029). That is NOT `char::is_whitespace`
/// (Unicode White_Space), in both directions: U+0085 NEL is White_Space
/// but not trimmed by JavaScript, and U+FEFF is trimmed by JavaScript but
/// not White_Space. The set is enumerated for exactly that reason.
fn is_trim_space(c: char) -> bool {
    matches!(
        c,
        '\u{0009}' // TAB
            | '\u{000A}' // LF
            | '\u{000B}' // VT
            | '\u{000C}' // FF
            | '\u{000D}' // CR
            | '\u{0020}' // SPACE
            | '\u{00A0}' // NO-BREAK SPACE
            | '\u{1680}' // OGHAM SPACE MARK
            | '\u{2000}' // EN QUAD
            | '\u{2001}' // EM QUAD
            | '\u{2002}' // EN SPACE
            | '\u{2003}' // EM SPACE
            | '\u{2004}' // THREE-PER-EM SPACE
            | '\u{2005}' // FOUR-PER-EM SPACE
            | '\u{2006}' // SIX-PER-EM SPACE
            | '\u{2007}' // FIGURE SPACE
            | '\u{2008}' // PUNCTUATION SPACE
            | '\u{2009}' // THIN SPACE
            | '\u{200A}' // HAIR SPACE
            | '\u{2028}' // LINE SEPARATOR
            | '\u{2029}' // PARAGRAPH SEPARATOR
            | '\u{202F}' // NARROW NO-BREAK SPACE
            | '\u{205F}' // MEDIUM MATHEMATICAL SPACE
            | '\u{3000}' // IDEOGRAPHIC SPACE
            | '\u{FEFF}' // ZWNBSP (BOM)
    )
}

/// JavaScript's `String.prototype.trim`, exactly: strip leading and
/// trailing code points in the ECMA-262 trim set.
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_trim_space)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_trim_follows_ecma_262_not_unicode_white_space() {
        assert_eq!(js_trim("\u{FEFF} hello \u{FEFF}"), "hello");
        assert_eq!(js_trim("\u{0085}hello\u{0085}"), "\u{0085}hello\u{0085}");
        assert_eq!(js_trim("\u{200B}hello"), "\u{200B}hello");
        assert_eq!(js_trim("\u{FEFF} \u{3000}"), "");
    }

    #[test]
    fn groups_enabled_mirrors_the_engine_filter() {
        assert!(groups_enabled("", "", ""));
        assert!(groups_enabled("mini", "mini", ""));
        assert!(!groups_enabled("", "mini", ""));
        assert!(!groups_enabled("mini", "mini", "mini"));
        assert!(groups_enabled("a,mini", "mini,json", "other"));
    }
}

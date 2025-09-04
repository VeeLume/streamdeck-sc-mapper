//! Activation mode parsing, deduping, and arena management (core-friendly).
//!
//! - `ActivationMode::resolve` finds/creates an index for the most specific mode
//!   described at a node, optionally falling back to a parent node.
//! - `ActivationArena` dedupes by `name` first (if present), then by a semantic key
//!   that quantizes float timings to milliseconds.
//!
//! This file is pure core: no plugin deps.

use roxmltree::Node;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed activation behavior for an input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ActivationMode {
    /// Optional symbolic name; when present it forms a stable identity in the arena.
    pub name: Option<String>,
    pub on_press: bool,
    pub on_hold: bool,
    pub on_release: bool,
    /// Number of taps to trigger the action (>= 1).
    pub multi_tap: i64,
    /// If true, block input while multi-tapping is being considered.
    pub multi_tap_block: bool,
    /// Threshold to consider a press as "triggered".
    pub press_trigger_threshold: Option<f32>,
    /// Threshold to consider a release as "triggered".
    pub release_trigger_threshold: Option<f32>,
    /// Delay after release until action is considered (seconds).
    pub release_trigger_delay: Option<f32>,
    /// Whether the action can retrigger without full reset.
    pub retriggerable: bool,
    /// Time you must hold before a hold triggers (seconds).
    pub hold_trigger_delay: Option<f32>,
    /// Repeat cadence while holding (seconds).
    pub hold_repeat_delay: Option<f32>,
}

impl ActivationMode {
    /// Parse attributes off `node`. If `include_name` is true, captures `name=""` as well.
    pub fn from_node(node: Node, include_name: bool) -> Self {
        #[inline]
        fn attr<'a>(node: Node<'a, 'a>, k: &'a str) -> Option<&'a str> {
            node.attribute(k)
        }
        #[inline]
        fn bool_attr(node: Node, k: &str) -> bool {
            attr(node, k) == Some("1")
        }
        #[inline]
        fn f32_attr_nonneg(node: Node, k: &str) -> Option<f32> {
            attr(node, k)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|&v| v.is_finite() && v >= 0.0)
        }
        #[inline]
        fn i64_attr_nonneg(node: Node, k: &str) -> Option<i64> {
            attr(node, k)
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|&v| v >= 0)
        }

        ActivationMode {
            name: if include_name {
                attr(node, "name").map(str::to_string)
            } else {
                None
            },
            on_press: bool_attr(node, "onPress"),
            on_hold: bool_attr(node, "onHold"),
            on_release: bool_attr(node, "onRelease"),
            multi_tap: i64_attr_nonneg(node, "multiTap").unwrap_or(1),
            multi_tap_block: bool_attr(node, "multiTapBlock"),
            press_trigger_threshold: f32_attr_nonneg(node, "pressTriggerThreshold"),
            release_trigger_threshold: f32_attr_nonneg(node, "releaseTriggerThreshold"),
            release_trigger_delay: f32_attr_nonneg(node, "releaseTriggerDelay"),
            retriggerable: bool_attr(node, "retriggerable"),
            hold_trigger_delay: f32_attr_nonneg(node, "holdTriggerDelay"),
            hold_repeat_delay: f32_attr_nonneg(node, "holdRepeatDelay"),
        }
    }

    /// Heuristic: does this node carry any activation-mode attributes?
    pub fn has_valid_attributes(node: Node) -> bool {
        const KEYS: &[&str] = &[
            "onPress",
            "onHold",
            "onRelease",
            "multiTap",
            "multiTapBlock",
            "pressTriggerThreshold",
            "releaseTriggerThreshold",
            "releaseTriggerDelay",
            "retriggerable",
            "holdTriggerDelay",
            "holdRepeatDelay",
        ];
        KEYS.iter().any(|&k| node.attribute(k).is_some())
    }

    /// Resolve a mode for `node` (and optional `fallback` node) into the arena.
    ///
    /// Order of precedence:
    /// 1. If `activationMode="<name>"` is present:
    ///    - If already known, return it.
    ///    - Else, try to *define* `<name>` from attributes at `node`,
    ///      or from `fallback` if the name is present there.
    /// 2. Else, if `node` has inline attributes, return/insert a semantic mode (no name).
    /// 3. Else, if `fallback` has inline attributes, use that.
    /// 4. Else, `None`.
    pub fn resolve(
        node: Node,
        fallback: Option<Node>,
        arena: &mut ActivationArena,
    ) -> Option<usize> {
        if let Some(mode_name) = node.attribute("activationMode") {
            if let Some(ix) = arena.find_by_name(mode_name) {
                return Some(ix);
            }
            // Define the named mode from the richest available attributes.
            let candidate = if Self::has_valid_attributes(node) {
                let mut m = Self::from_node(node, true);
                m.name = Some(mode_name.to_string());
                m
            } else if let Some(f) = fallback.filter(|n| Self::has_valid_attributes(*n)) {
                let mut m = Self::from_node(f, true);
                m.name = Some(mode_name.to_string());
                m
            } else {
                // Named with no definition anywhere ⇒ unknown reference.
                return None;
            };
            return Some(arena.insert_or_get_mode(candidate));
        }

        // No explicit name: try inline attributes, then fallback’s inline attributes.
        if Self::has_valid_attributes(node) {
            let m = Self::from_node(node, false);
            return Some(arena.insert_or_get_mode(m));
        }

        if let Some(f) = fallback.filter(|n| Self::has_valid_attributes(*n)) {
            let m = Self::from_node(f, false);
            return Some(arena.insert_or_get_mode(m));
        }

        None
    }

    /// Convenience: keep older call sites working.
    #[inline]
    pub fn insert_or_get(arena: &mut ActivationArena, mode: ActivationMode) -> usize {
        arena.insert_or_get_mode(mode)
    }
}

/// Quantized semantic key for dedupe (convert seconds → milliseconds, round to nearest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ModeKey {
    on_press: bool,
    on_hold: bool,
    on_release: bool,
    multi_tap: i64,
    multi_tap_block: bool,
    press_ms: Option<u32>,
    release_thr_ms: Option<u32>,
    release_delay_ms: Option<u32>,
    retriggerable: bool,
    hold_ms: Option<u32>,
    hold_repeat_ms: Option<u32>,
}

impl ModeKey {
    #[inline]
    fn quantize_ms(x: Option<f32>) -> Option<u32> {
        x.map(|v| {
            // robust to weird inputs
            let v = if v.is_finite() { v } else { 0.0 };
            (v * 1000.0).round().max(0.0) as u32
        })
    }

    #[inline]
    fn from(m: &ActivationMode) -> Self {
        ModeKey {
            on_press: m.on_press,
            on_hold: m.on_hold,
            on_release: m.on_release,
            multi_tap: m.multi_tap,
            multi_tap_block: m.multi_tap_block,
            press_ms: Self::quantize_ms(m.press_trigger_threshold),
            release_thr_ms: Self::quantize_ms(m.release_trigger_threshold),
            release_delay_ms: Self::quantize_ms(m.release_trigger_delay),
            retriggerable: m.retriggerable,
            hold_ms: Self::quantize_ms(m.hold_trigger_delay),
            hold_repeat_ms: Self::quantize_ms(m.hold_repeat_delay),
        }
    }
}

/// Arena of deduped activation modes.
///
/// Serialization note:
/// - `modes` is serialized; the lookup maps are rebuilt at runtime via `rebuild_indexes()`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivationArena {
    /// Canonical list of modes (serialize only this).
    pub modes: Vec<ActivationMode>,

    #[serde(skip)]
    name_to_index: HashMap<String, usize>,
    #[serde(skip)]
    by_key: HashMap<ModeKey, usize>,
}

impl ActivationArena {
    #[inline]
    pub fn len(&self) -> usize {
        self.modes.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.modes.is_empty()
    }

    #[inline]
    pub fn get(&self, ix: usize) -> Option<&ActivationMode> {
        self.modes.get(ix)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (usize, &ActivationMode)> {
        self.modes.iter().enumerate()
    }

    #[inline]
    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    /// Insert or return an existing index for a mode.
    ///
    /// Dedupe order:
    /// 1) If `name` exists, it wins and maps to a single index.
    /// 2) Else, dedupe by semantic key (`ModeKey`).
    pub fn insert_or_get_mode(&mut self, m: ActivationMode) -> usize {
        if let Some(name) = m.name.as_deref() {
            if let Some(ix) = self.name_to_index.get(name) {
                return *ix;
            }
        }

        let key = ModeKey::from(&m);
        if let Some(ix) = self.by_key.get(&key) {
            // If the existing semantic entry didn’t have a name and this one does, remember it.
            if let Some(name) = m.name {
                self.name_to_index.entry(name).or_insert(*ix);
            }
            return *ix;
        }

        let ix = self.modes.len();
        if let Some(name) = m.name.as_deref() {
            self.name_to_index.insert(name.to_string(), ix);
        }
        self.by_key.insert(key, ix);
        self.modes.push(m);
        ix
    }

    /// Rebuild lookup maps (call after deserialization or bulk edits).
    pub fn rebuild_indexes(&mut self) {
        self.name_to_index.clear();
        self.by_key.clear();
        for (ix, m) in self.modes.iter().enumerate() {
            if let Some(name) = m.name.as_deref() {
                self.name_to_index.insert(name.to_string(), ix);
            }
            self.by_key.insert(ModeKey::from(m), ix);
        }
    }
}

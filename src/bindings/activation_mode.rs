use roxmltree::Node;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Your ActivationMode as before
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivationMode {
    pub name: Option<String>,
    pub on_press: bool,
    pub on_hold: bool,
    pub on_release: bool,
    pub multi_tap: i64,
    pub multi_tap_block: bool,
    pub press_trigger_threshold: Option<f32>,
    pub release_trigger_threshold: Option<f32>,
    pub release_trigger_delay: Option<f32>,
    pub retriggerable: bool,
    pub hold_trigger_delay: Option<f32>,
    pub hold_repeat_delay: Option<f32>,
}

impl ActivationMode {
    pub fn from_node(node: Node, include_name: bool) -> Self {
        let attr = |k: &str| node.attribute(k);
        let bool_attr = |k: &str| attr(k) == Some("1");
        let f32_attr = |k: &str| {
            attr(k)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|&v| v >= 0.0)
        };
        let i64_attr = |k: &str| {
            attr(k)
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|&v| v >= 0)
        };

        ActivationMode {
            name: if include_name {
                attr("name").map(str::to_string)
            } else {
                None
            },
            on_press: bool_attr("onPress"),
            on_hold: bool_attr("onHold"),
            on_release: bool_attr("onRelease"),
            multi_tap: i64_attr("multiTap").unwrap_or(1),
            multi_tap_block: bool_attr("multiTapBlock"),
            press_trigger_threshold: f32_attr("pressTriggerThreshold"),
            release_trigger_threshold: f32_attr("releaseTriggerThreshold"),
            release_trigger_delay: f32_attr("releaseTriggerDelay"),
            retriggerable: bool_attr("retriggerable"),
            hold_trigger_delay: f32_attr("holdTriggerDelay"),
            hold_repeat_delay: f32_attr("holdRepeatDelay"),
        }
    }

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

    /// Resolve a mode for a node (and optional fallback node) into the arena. Returns arena index.
    pub fn resolve(
        node: Node,
        fallback: Option<Node>,
        arena: &mut ActivationArena,
    ) -> Option<usize> {
        // Named reference first
        if let Some(mode_name) = node.attribute("activationMode") {
            if let Some(ix) = arena.find_by_name(mode_name) {
                return Some(ix);
            }
            let candidate = if Self::has_valid_attributes(node) {
                let mut m = Self::from_node(node, true);
                m.name = Some(mode_name.to_string());
                m
            } else if let Some(f) = fallback.filter(|n| Self::has_valid_attributes(*n)) {
                let mut m = Self::from_node(f, true);
                m.name = Some(mode_name.to_string());
                m
            } else {
                return None;
            };
            return Some(arena.insert_or_get_mode(candidate));
        }

        // Inline attributes w/o name
        if Self::has_valid_attributes(node) {
            let m = Self::from_node(node, false);
            return Some(arena.insert_or_get_mode(m));
        }

        // Fallback node
        if let Some(f) = fallback.filter(|n| Self::has_valid_attributes(*n)) {
            let m = Self::from_node(f, false);
            return Some(arena.insert_or_get_mode(m));
        }

        None
    }

    /// Keep your existing call sites working:
    pub fn insert_or_get(arena: &mut ActivationArena, mode: ActivationMode) -> usize {
        arena.insert_or_get_mode(mode)
    }
}

/// Quantized semantic key for dedupe (convert seconds -> milliseconds, round).
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
    fn quantize_ms(x: Option<f32>) -> Option<u32> {
        x.map(|v| (v * 1000.0).round().max(0.0) as u32)
    }
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

/// Arena of deduped activation modes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivationArena {
    /// The canonical list (serialize only this).
    pub modes: Vec<ActivationMode>,

    #[serde(skip)]
    name_to_index: HashMap<String, usize>,
    #[serde(skip)]
    by_key: HashMap<ModeKey, usize>,
}

impl ActivationArena {
    pub fn len(&self) -> usize {
        self.modes.len()
    }
    pub fn get(&self, ix: usize) -> Option<&ActivationMode> {
        self.modes.get(ix)
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &ActivationMode)> {
        self.modes.iter().enumerate()
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    /// Insert or return existing index for a mode (dedupe by name, then by semantics).
    pub fn insert_or_get_mode(&mut self, m: ActivationMode) -> usize {
        if let Some(name) = m.name.as_deref() {
            if let Some(ix) = self.name_to_index.get(name) {
                return *ix;
            }
        }
        let key = ModeKey::from(&m);
        if let Some(ix) = self.by_key.get(&key) {
            // If it has a new name and existing one didn’t, keep the existing index but remember the name.
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

    /// Rebuild hash maps after (de)serialization or bulk edits.
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

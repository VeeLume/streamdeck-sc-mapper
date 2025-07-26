use roxmltree::Node;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MultiTap {
    One,
    Two,
}

impl MultiTap {
    pub fn from_str(value: &str) -> Self {
        match value {
            "2" => MultiTap::Two,
            _ => MultiTap::One,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivationMode {
    pub name: Option<String>,
    pub on_press: bool,
    pub on_hold: bool,
    pub on_release: bool,
    pub multi_tap: MultiTap,
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
        let attr = |key: &str| node.attribute(key);
        let bool_attr = |key: &str| attr(key) == Some("1");
        let f32_attr = |key: &str| attr(key).and_then(|v| v.parse::<f32>().ok()).filter(|&v| v >= 0.0);

        ActivationMode {
            name: if include_name { attr("name").map(str::to_string) } else { None },
            on_press: bool_attr("onPress"),
            on_hold: bool_attr("onHold"),
            on_release: bool_attr("onRelease"),
            multi_tap: MultiTap::from_str(attr("multiTap").unwrap_or("1")),
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
        let keys = [
            "onPress", "onHold", "onRelease", "multiTap", "multiTapBlock",
            "pressTriggerThreshold", "releaseTriggerThreshold", "releaseTriggerDelay",
            "retriggerable", "holdTriggerDelay", "holdRepeatDelay",
        ];
        keys.iter().any(|&k| node.attribute(k).is_some())
    }

    pub fn resolve(
        node: Node,
        fallback: Option<Node>,
        activation_modes: &[ActivationMode],
    ) -> Option<ActivationMode> {
        Self::resolve_from_attr(node, activation_modes)
            .or_else(|| if Self::has_valid_attributes(node) {
                Some(Self::from_node(node, false))
            } else {
                fallback.and_then(|f| {
                    Self::resolve_from_attr(f, activation_modes)
                        .or_else(|| if Self::has_valid_attributes(f) {
                            Some(Self::from_node(f, false))
                        } else {
                            None
                        })
                })
            })
    }

    fn resolve_from_attr<'a>(
        node: Node,
        activation_modes: &'a [ActivationMode],
    ) -> Option<ActivationMode> {
        let mode_name = node.attribute("activationMode")?;
        activation_modes
            .iter()
            .find(|am| am.name.as_deref() == Some(mode_name))
            .cloned()
    }
}

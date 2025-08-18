// src/actions/sc_action.rs
use std::{ sync::{ atomic::{ AtomicBool, Ordering }, Arc }, thread, time::{ Duration, Instant } };
use constcat::concat;
use serde::{ Deserialize, Serialize };
use serde_json::{ json, Map, Value };
use streamdeck_lib::prelude::*;

use crate::{
    bindings::{ action_bindings::ActionBindingsStore },
    data_source::{ DataSourceResult, Item, ItemGroup },
    sc::{
        adapters::bindings_adapter::{ load_translations },
        shared::{ ResourceDir },
        topics::{ ExecSend },
    },
    serde_helpers::{ opt_u64_from_str_or_num, u64_from_str_or_num_default_200 },
};
use crate::sc::topics::{ ACTIONS_CACHE_UPDATED, EXEC_SEND };
use crate::PLUGIN_ID;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScActionSettings {
    #[serde(rename = "actionShort", default)]
    short_id: Option<String>,
    #[serde(rename = "actionShortHold", deserialize_with = "opt_u64_from_str_or_num", default)]
    short_hold_ms: Option<u64>,
    #[serde(rename = "actionLong", default)]
    long_id: Option<String>,
    #[serde(rename = "actionLongHold", deserialize_with = "opt_u64_from_str_or_num", default)]
    long_hold_ms: Option<u64>,

    #[serde(
        default = "ScActionSettings::default_long_threshold",
        rename = "longPressPeriod",
        deserialize_with = "u64_from_str_or_num_default_200"
    )]
    long_threshold_ms: u64,
}

impl ScActionSettings {
    fn default_long_threshold() -> u64 {
        200
    }

    /// Parse from a borrowed settings map
    fn from_map(map: &Map<String, Value>) -> serde_json::Result<Self> {
        serde_json::from_value(Value::Object(map.clone()))
    }
}

#[derive(Default)]
pub struct ScAction {
    // runtime
    down_at: Option<Instant>,
    // long timer control
    long_cancel: Arc<AtomicBool>,
    long_fired: Arc<AtomicBool>,
    // if we fired short on key_down (when no long is configured)
    short_fired_on_down: bool,
}

impl ActionStatic for ScAction {
    const ID: &'static str = concat!(PLUGIN_ID, ".sc-action");
}

impl Action for ScAction {
    fn id(&self) -> &str {
        Self::ID
    }
    fn topics(&self) -> &'static [&'static str] {
        &[ACTIONS_CACHE_UPDATED.name]
    }

    fn init(&mut self, cx: &Context, ctx: &str) {
        info!(cx.log(), "ScAction init for {}", ctx);
    }

    fn did_receive_property_inspector_message(
        &mut self,
        cx: &Context,
        ev: &DidReceivePropertyInspectorMessage
    ) {
        debug!(cx.log(), "Received PI message: context={}, message={:?}", ev.context, ev.payload);
        // Expect payload: { event: "getActions", isRefresh?: true }
        let ev_name = ev.payload
            .get("event")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if ev_name != "getActions" {
            return;
        }

        build_pi_items(cx, ev.context);
    }

    fn will_appear(&mut self, _cx: &Context, _ev: &WillAppear) {
        self.down_at = None;
        self.long_cancel = Arc::new(AtomicBool::new(false));
        self.long_fired = Arc::new(AtomicBool::new(false));
        self.short_fired_on_down = false;
    }

    fn key_down(&mut self, cx: &Context, ev: &KeyDown) {
        self.down_at = Some(Instant::now());
        self.long_cancel.store(false, Ordering::SeqCst);
        self.long_fired.store(false, Ordering::SeqCst);
        self.short_fired_on_down = false;

        let settings = match ScActionSettings::from_map(ev.settings) {
            Ok(s) => s,
            Err(e) => {
                error!(cx.log(), "Failed to parse action settings: {}", e);
                return;
            }
        };

        debug!(
            cx.log(),
            "key_down: action={} context={}, short={:?}({:?}ms) long={:?}({:?}ms)",
            self.id(),
            ev.context,
            settings.short_id,
            settings.short_hold_ms.unwrap_or(0),
            settings.long_id,
            settings.long_hold_ms.unwrap_or(0)
        );

        // If no long action is configured, fire short immediately.
        if settings.long_id.is_none() {
            if let Some(id) = settings.short_id.as_deref() {
                debug!(cx.log(), "key_down: firing short action '{}' immediately", id);
                cx.bus().adapters_notify_topic_t(EXEC_SEND, None, ExecSend {
                    action_id: id.to_string(),
                    hold_ms: settings.short_hold_ms,
                    is_down: None, // normal key press
                });
                cx.sd().show_ok(ev.context);
                self.short_fired_on_down = true;
            }
            return;
        }

        // -------- everything below is owned/'static for the spawned thread --------
        let threshold_ms = settings.long_threshold_ms;
        let cancel = self.long_cancel.clone();
        let long_fired = self.long_fired.clone();

        let ctx = cx.clone(); // Context is Clone + 'static in your framework
        let ctx_id: String = ev.context.to_string(); // OWN the context id
        let long_id: String = settings.long_id.clone().unwrap(); // safe: checked above
        let long_hold = settings.long_hold_ms;

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(threshold_ms));
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            long_fired.store(true, Ordering::SeqCst);
            debug!(
                ctx.log(),
                "key_down: firing long action '{}' after {}ms",
                long_id,
                threshold_ms
            );
            ctx.bus().adapters_notify_topic_t(EXEC_SEND, None, ExecSend {
                action_id: long_id,
                hold_ms: long_hold,
                is_down: None, // normal key press
            });
            ctx.sd().show_ok(ctx_id);
        });
    }

    fn key_up(&mut self, cx: &Context, ev: &KeyUp) {
        debug!(cx.log(), "key_up: action={} context={}", self.id(), ev.context);

        // cancel any pending long
        self.long_cancel.store(true, Ordering::SeqCst);

        // if long already fired while held, we're done
        if self.long_fired.load(Ordering::SeqCst) {
            return;
        }

        // if we already fired short on key_down (because no long was configured), do nothing
        if self.short_fired_on_down {
            return;
        }

        let settings = match ScActionSettings::from_map(ev.settings) {
            Ok(s) => s,
            Err(e) => {
                error!(cx.log(), "Failed to parse action settings: {}", e);
                return;
            }
        };

        // long was configured but threshold not reached ⇒ short (if configured)
        if let Some(id) = settings.short_id.as_deref() {
            debug!(
                cx.log(),
                "key_up: firing short action '{}' after {}ms",
                id,
                settings.short_hold_ms.unwrap_or(0)
            );
            cx.bus().adapters_notify_topic_t(EXEC_SEND, None, ExecSend {
                action_id: id.to_string(),
                hold_ms: settings.short_hold_ms,
                is_down: None, // normal key press
            });
            cx.sd().show_ok(ev.context);
        }
    }
}

fn build_pi_items(cx: &Context, cx_id: &str) {
    let resource_dir = match cx.try_ext::<ResourceDir>() {
        Some(dir) => dir.get(),
        None => {
            error!(cx.log(), "ResourceDir ext missing, cannot get resource directory");
            return;
        }
    };
    let action_store = match cx.try_ext::<ActionBindingsStore>() {
        Some(store) => store,
        None => {
            error!(cx.log(), "ActionBindingsStore ext missing, cannot get actions");
            return;
        }
    };
    let bindings = action_store.snapshot();
    let translations = load_translations(resource_dir.join("global.ini"), &cx.log());
    let mut items = vec![DataSourceResult::Item(Item::with_label("", "No Action"))];
    items.extend(
        bindings.action_maps.values().map(|am| {
            let children: Vec<Item> = am.actions
                .values()
                .map(|ab| {
                    Item::with_label(
                        ab.action_id.to_string(),
                        format!(
                            "{} [{}]",
                            ab.get_label(&translations),
                            ab.get_binds_label().unwrap_or_default()
                        )
                    )
                })
                .collect();
            DataSourceResult::ItemGroup(ItemGroup::new(am.get_label(&translations), children))
        })
    );
    cx.sd().send_to_property_inspector(
        cx_id,
        json!({
            "event": "getActions",
            "items": items,
        })
    );
}

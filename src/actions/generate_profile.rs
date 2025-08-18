// src/actions/generate_profile.rs
use chrono::Local;
use constcat::concat;
use std::time::{Duration, Instant};
use streamdeck_lib::prelude::*;

use crate::PLUGIN_ID;
use crate::sc::adapters::bindings_adapter::BindingsAdapter;
use crate::sc::shared::{ActiveInstall, GameInstallType};
use crate::sc::topics::{BINDINGS_REBUILD_AND_SAVE, BindingsRebuildAndSave};

pub struct GenerateProfileAction {
    down_at: Option<Instant>,
    long_ms: u64, // threshold (press >= long_ms => without custom)
}

impl Default for GenerateProfileAction {
    fn default() -> Self {
        Self {
            down_at: None,
            long_ms: 500, // sensible default
        }
    }
}

impl ActionStatic for GenerateProfileAction {
    const ID: &'static str = concat!(PLUGIN_ID, ".generate-profile");
}

impl Action for GenerateProfileAction {
    fn id(&self) -> &str {
        Self::ID
    }

    fn init(&mut self, cx: &Context, ctx_id: &str) {
        info!(cx.log(), "GenerateProfileAction init: {}", ctx_id);
        // keep the default unless you want to override from globals later
        // self.long_ms = 500;
    }

    fn will_appear(&mut self, _cx: &Context, _ev: &WillAppear) {
        self.down_at = None;
    }

    fn key_down(&mut self, _cx: &Context, _ev: &KeyDown) {
        self.down_at = Some(Instant::now());
    }

    fn key_up(&mut self, cx: &Context, ev: &KeyUp) {
        let held_ms = self
            .down_at
            .take()
            .and_then(|t| Instant::now().checked_duration_since(t))
            .unwrap_or(Duration::from_millis(0))
            .as_millis() as u64;

        // short → with custom (true), long → without custom (false)
        let with_custom = held_ms < self.long_ms;

        let ty = match cx.try_ext::<ActiveInstall>() {
            Some(a) => a.get(),
            None => GameInstallType::Live, // Default to Live if not set
        };
        // Profile name is Plugin ID + install type + timestamp
        let profile_name = Some(format!(
            "{}-{}-{}",
            "SC-Mapper",
            ty.name(),
            Local::now().format("%Y.%m.%d-%H:%M")
        ));

        info!(
            cx.log(),
            "generate-profile press={}ms with_custom={} ty={:?}", held_ms, with_custom, ty
        );

        cx.bus().adapters_notify_name_of::<BindingsAdapter, _>(
            BINDINGS_REBUILD_AND_SAVE,
            None,
            BindingsRebuildAndSave {
                ty,
                with_custom,
                name: profile_name.clone(),
            },
        );

        cx.sd().show_ok(ev.context);
    }
}

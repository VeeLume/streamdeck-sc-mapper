use std::{ collections::HashMap };
use serde_json::Value;
use websocket::{ OwnedMessage };

use crate::{ data_source::DataSourcePayload, plugin::WriteSink };

pub mod generate_binds;
pub mod sc_action;

#[derive(Debug, Clone)]
pub struct KeyCoordinates {
    pub column: i32,
    pub row: i32,
}

pub trait ActionHandler: Send + Sync {
    fn on_message(&self, write: WriteSink, msg: &HashMap<String, Value>) {
        let Some(event) = msg.get("event").and_then(Value::as_str) else {
            return;
        };

        let context = msg.get("context").and_then(Value::as_str).unwrap_or_default();
        let payload = msg.get("payload");

        match event {
            "dialDown" => {
                if
                    let (Some(device), Some(coords), Some(settings)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("coordinates")),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                    )
                {
                    let coordinates = KeyCoordinates {
                        column: coords.get("column").and_then(Value::as_i64).unwrap_or(0) as i32,
                        row: coords.get("row").and_then(Value::as_i64).unwrap_or(0) as i32,
                    };
                    self.on_dial_down(write, context, device, &coordinates, settings);
                }
            }

            "dialRotate" => {
                if
                    let (Some(device), Some(coords), Some(settings), Some(pressed), Some(ticks)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("coordinates")),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                        payload.and_then(|p| p.get("pressed")).and_then(Value::as_bool),
                        payload.and_then(|p| p.get("ticks")).and_then(Value::as_f64),
                    )
                {
                    let coordinates = KeyCoordinates {
                        column: coords.get("column").and_then(Value::as_i64).unwrap_or(0) as i32,
                        row: coords.get("row").and_then(Value::as_i64).unwrap_or(0) as i32,
                    };
                    self.on_dial_rotate(
                        write,
                        "",
                        context,
                        device,
                        &coordinates,
                        pressed,
                        settings,
                        ticks
                    );
                }
            }

            "dialUp" => {
                if
                    let (Some(device), Some(coords), Some(settings)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("coordinates")),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                    )
                {
                    let coordinates = KeyCoordinates {
                        column: coords.get("column").and_then(Value::as_i64).unwrap_or(0) as i32,
                        row: coords.get("row").and_then(Value::as_i64).unwrap_or(0) as i32,
                    };
                    self.on_dial_up(write, context, device, &coordinates, settings);
                }
            }

            "sendToPlugin" => {
                if let Some(payload) = payload {
                    self.on_did_receive_property_inspector_message(write, "", context, payload);
                }
            }

            "didReceiveSettings" => {
                if
                    let (Some(device), Some(controller), Some(settings), Some(is_multi)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("controller")).and_then(Value::as_str),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                        payload.and_then(|p| p.get("isInMultiAction")).and_then(Value::as_bool),
                    )
                {
                    let coordinates = payload
                        .and_then(|p| p.get("coordinates"))
                        .and_then(|coords| {
                            Some(KeyCoordinates {
                                column: coords.get("column")?.as_i64()? as i32,
                                row: coords.get("row")?.as_i64()? as i32,
                            })
                        });

                    let state = payload
                        .and_then(|p| p.get("state"))
                        .and_then(Value::as_u64)
                        .map(|s| s as u8);

                    self.on_did_receive_settings(
                        write,
                        "",
                        context,
                        controller,
                        is_multi,
                        coordinates.as_ref(),
                        settings,
                        state
                    );
                }
            }

            "keyDown" | "keyUp" => {
                if
                    let (Some(device), Some(settings), Some(is_multi)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                        payload.and_then(|p| p.get("isInMultiAction")).and_then(Value::as_bool),
                    )
                {
                    let coordinates = payload
                        .and_then(|p| p.get("coordinates"))
                        .and_then(|coords| {
                            Some(KeyCoordinates {
                                column: coords.get("column")?.as_i64()? as i32,
                                row: coords.get("row")?.as_i64()? as i32,
                            })
                        });

                    let state = payload
                        .and_then(|p| p.get("state"))
                        .and_then(Value::as_u64)
                        .map(|s| s as u8);
                    let user_state = payload
                        .and_then(|p| p.get("userDesiredState"))
                        .and_then(Value::as_u64)
                        .map(|s| s as u8);

                    match event {
                        "keyDown" => {
                            self.on_key_down(
                                write,
                                "",
                                context,
                                device,
                                is_multi,
                                coordinates.as_ref(),
                                settings,
                                state,
                                user_state
                            );
                        }
                        "keyUp" => {
                            self.on_key_up(
                                write,
                                "",
                                context,
                                device,
                                is_multi,
                                coordinates.as_ref(),
                                settings,
                                state
                            );
                        }
                        _ => {}
                    }
                }
            }

            "propertyInspectorDidAppear" => {
                if let Some(device) = msg.get("device").and_then(Value::as_str) {
                    self.on_property_inspector_did_appear(write, "", context, device);
                }
            }

            "propertyInspectorDidDisappear" => {
                if let Some(device) = msg.get("device").and_then(Value::as_str) {
                    self.on_property_inspector_did_disappear(write, "", context, device);
                }
            }

            "titleParametersDidChange" => {
                if
                    let (
                        Some(device),
                        Some(controller),
                        Some(coords),
                        Some(settings),
                        Some(title),
                        Some(params),
                    ) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("controller")).and_then(Value::as_str),
                        payload.and_then(|p| p.get("coordinates")),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                        payload.and_then(|p| p.get("title")).and_then(Value::as_str),
                        payload.and_then(|p| p.get("titleParameters")).and_then(Value::as_object),
                    )
                {
                    let coordinates = KeyCoordinates {
                        column: coords.get("column").and_then(Value::as_i64).unwrap_or(0) as i32,
                        row: coords.get("row").and_then(Value::as_i64).unwrap_or(0) as i32,
                    };
                    self.on_title_parameters_did_change(
                        write,
                        "",
                        context,
                        device,
                        controller,
                        &coordinates,
                        settings,
                        payload
                            .and_then(|p| p.get("state"))
                            .and_then(Value::as_u64)
                            .map(|s| s as u8),
                        title,
                        params.get("fontFamily").and_then(Value::as_str).unwrap_or_default(),
                        params.get("fontSize").and_then(Value::as_i64).unwrap_or(12) as i32,
                        params.get("fontStyle").and_then(Value::as_str).unwrap_or_default(),
                        params.get("fontUnderline").and_then(Value::as_bool).unwrap_or(false),
                        params.get("showTitle").and_then(Value::as_bool).unwrap_or(true),
                        params.get("titleAlignment").and_then(Value::as_str).unwrap_or_default(),
                        params.get("titleColor").and_then(Value::as_str).unwrap_or_default()
                    );
                }
            }

            "touchTap" => {
                if
                    let (Some(device), Some(coords), Some(settings), Some(tab_pos)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("coordinates")),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                        payload.and_then(|p| p.get("tapPos")).and_then(Value::as_array),
                    )
                {
                    let coordinates = KeyCoordinates {
                        column: coords.get("column").and_then(Value::as_i64).unwrap_or(0) as i32,
                        row: coords.get("row").and_then(Value::as_i64).unwrap_or(0) as i32,
                    };
                    let pos = (
                        tab_pos.get(0).and_then(Value::as_f64).unwrap_or(0.0),
                        tab_pos.get(1).and_then(Value::as_f64).unwrap_or(0.0),
                    );
                    let hold = payload
                        .and_then(|p| p.get("hold"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    self.on_touch_tab(
                        write,
                        "",
                        context,
                        device,
                        &coordinates,
                        hold,
                        settings,
                        pos
                    );
                }
            }

            "willAppear" | "willDisappear" => {
                if
                    let (Some(device), Some(controller), Some(settings), Some(is_multi)) = (
                        msg.get("device").and_then(Value::as_str),
                        payload.and_then(|p| p.get("controller")).and_then(Value::as_str),
                        payload.and_then(|p| p.get("settings")).and_then(Value::as_object),
                        payload.and_then(|p| p.get("isInMultiAction")).and_then(Value::as_bool),
                    )
                {
                    let coordinates = payload
                        .and_then(|p| p.get("coordinates"))
                        .and_then(|coords| {
                            Some(KeyCoordinates {
                                column: coords.get("column")?.as_i64()? as i32,
                                row: coords.get("row")?.as_i64()? as i32,
                            })
                        });

                    let state = payload
                        .and_then(|p| p.get("state"))
                        .and_then(Value::as_u64)
                        .map(|s| s as u8);

                    match event {
                        "willAppear" => {
                            self.on_will_appear(
                                write,
                                "",
                                context,
                                device,
                                is_multi,
                                coordinates.as_ref(),
                                settings,
                                state
                            );
                        }
                        "willDisappear" => {
                            self.on_will_disappear(
                                write,
                                "",
                                context,
                                device,
                                is_multi,
                                coordinates.as_ref(),
                                settings,
                                state
                            );
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }

    fn on_dial_down(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        coordinates: &KeyCoordinates,
        settings: &serde_json::Map<std::string::String, Value>
    ) {}

    fn on_dial_rotate(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        coordinates: &KeyCoordinates,
        pressed: bool,
        settings: &serde_json::Map<std::string::String, Value>,
        ticks: f64
    ) {}

    fn on_dial_up(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        coordinates: &KeyCoordinates,
        settings: &serde_json::Map<std::string::String, Value>
    ) {}

    fn on_did_receive_property_inspector_message(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        payload: &Value
    ) {}

    fn on_did_receive_settings(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        controller: &str,
        is_in_multi_action: bool,
        coordinates: Option<&KeyCoordinates>,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>
    ) {}

    fn on_key_down(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        is_in_multi_action: bool,
        coordinates: Option<&KeyCoordinates>,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>,
        user_desired_state: Option<u8>
    ) {}

    fn on_key_up(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        is_in_multi_action: bool,
        coordinates: Option<&KeyCoordinates>,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>
    ) {}

    fn on_property_inspector_did_appear(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str
    ) {}

    fn on_property_inspector_did_disappear(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str
    ) {}

    fn on_title_parameters_did_change(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        controller: &str,
        coordinates: &KeyCoordinates,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>,
        title: &str,
        font_family: &str,
        font_size: i32,
        font_style: &str,
        font_underline: bool,
        show_title: bool,
        title_alignment: &str,
        title_color: &str
    ) {}

    fn on_touch_tab(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        coordinates: &KeyCoordinates,
        hold: bool,
        settings: &serde_json::Map<std::string::String, Value>,
        tap_pos: (f64, f64)
    ) {}

    fn on_will_appear(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        is_in_multi_action: bool,
        coordinates: Option<&KeyCoordinates>,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>
    ) {}

    fn on_will_disappear(
        &self,
        write: WriteSink,
        action: &str,
        context: &str,
        device: &str,
        is_in_multi_action: bool,
        coordinates: Option<&KeyCoordinates>,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>
    ) {}
}

fn get_setting(write: WriteSink, context: &str) {
    let msg = serde_json::json!({
        "event": "getSettings",
        "context": context,
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn send_to_property_inspector(write: WriteSink, context: &str, payload: DataSourcePayload) {
    let msg =
        serde_json::json!({
        "event": "sendToPropertyInspector",
        "context": context,
        "payload": payload
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_feedback(write: WriteSink, context: &str, layout: HashMap<String, Value>) {
    let msg =
        serde_json::json!({
        "event": "setFeedback",
        "context": context,
        "layout": layout
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_feedback_layout(write: WriteSink, context: &str, layout: &str) {
    let msg =
        serde_json::json!({
        "event": "setFeedbackLayout",
        "context": context,
        "layout": layout
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_image(
    write: WriteSink,
    context: &str,
    image: Option<String>,
    state: Option<u8>,
    target: Option<String>
) {
    let msg =
        serde_json::json!({
        "event": "setImage",
        "context": context,
        "payload": {
            "image": image,
            "state": state,
            "target": target
        }
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_settings(write: WriteSink, context: &str, settings: HashMap<String, Value>) {
    let msg =
        serde_json::json!({
        "event": "setSettings",
        "context": context,
        "payload": settings
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_state(write: WriteSink, context: &str, state: u8) {
    let msg =
        serde_json::json!({
        "event": "setState",
        "context": context,
        "payload": {
            "state": state
        }
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_title(
    write: WriteSink,
    context: &str,
    state: Option<u8>,
    target: Option<String>,
    title: Option<String>
) {
    let msg =
        serde_json::json!({
        "event": "setTitle",
        "context": context,
        "payload": {
            "title": title,
            "state": state,
            "target": target
        }
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn set_trigger_description(
    write: WriteSink,
    context: &str,
    long_touch: Option<String>,
    push: Option<String>,
    rotate: Option<String>,
    touch: Option<String>
) {
    let msg =
        serde_json::json!({
        "event": "setTriggerDescription",
        "context": context,
        "payload": {
            "longTouch": long_touch,
            "push": push,
            "rotate": rotate,
            "touch": touch
        }
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn show_alert(write: WriteSink, context: &str) {
    let msg = serde_json::json!({
        "event": "showAlert",
        "context": context
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

fn show_ok(write: WriteSink, context: &str) {
    let msg = serde_json::json!({
        "event": "showOk",
        "context": context
    });

    let msg = OwnedMessage::Text(msg.to_string());
    if let Ok(mut writer) = write.lock() {
        let _ = writer.send_message(&msg);
    }
}

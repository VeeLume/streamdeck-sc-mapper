use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::bindings::action_bindings::ActionBindings;
use crate::bindings::bind::BindOrigin;
use crate::bindings::bind_tokens::bind_to_input_with_prefix;

impl ActionBindings {
    /// Emit a Star Citizen mappings XML containing **only** actions that have active custom binds.
    ///
    /// - `devices`: optional list of (`"keyboard"|"mouse"`, instance_id_str). Defaults to `keyboard=1, mouse=1`.
    /// - `profile_name`: written into `<ActionMaps profileName="">` and `<CustomisationUIHeader label="">`.
    pub fn generate_mapping_xml<P: AsRef<Path>>(
        &self,
        output_path: P,
        devices: Option<&[(&str, &str)]>,
        profile_name: &str,
    ) -> Result<(), String> {
        // ── writer setup ─────────────────────────────────────────────────────────
        let file = File::create(&output_path)
            .map_err(|e| format!("create {}: {e}", output_path.as_ref().display()))?;
        let mut writer = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

        // XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .map_err(|e| format!("xml decl: {e}"))?;

        // ── <ActionMaps ...> ─────────────────────────────────────────────────────
        let mut root = BytesStart::new("ActionMaps");
        root.push_attribute(("version", "1"));
        root.push_attribute(("optionsVersion", "2"));
        root.push_attribute(("rebindVersion", "2"));
        root.push_attribute(("profileName", profile_name));
        writer
            .write_event(Event::Start(root))
            .map_err(|e| format!("<ActionMaps>: {e}"))?;

        // ── <CustomisationUIHeader ...> ──────────────────────────────────────────
        let mut header = BytesStart::new("CustomisationUIHeader");
        header.push_attribute(("label", profile_name));
        header.push_attribute(("description", ""));
        header.push_attribute(("image", ""));
        writer
            .write_event(Event::Start(header))
            .map_err(|e| format!("<CustomisationUIHeader>: {e}"))?;

        // ── <devices> (defaults: keyboard=1, mouse=1) ───────────────────────────
        writer
            .write_event(Event::Start(BytesStart::new("devices")))
            .map_err(|e| format!("<devices>: {e}"))?;

        let default_devices = [("keyboard", "1"), ("mouse", "1")];
        let dev_list = devices.unwrap_or(&default_devices);

        // Resolve instance ids we’ll use in the rebind "input" strings
        let kb_inst = dev_list
            .iter()
            .find(|(t, _)| *t == "keyboard")
            .map(|(_, i)| *i)
            .unwrap_or("1");
        let mo_inst = dev_list
            .iter()
            .find(|(t, _)| *t == "mouse")
            .map(|(_, i)| *i)
            .unwrap_or("1");

        for &(dev_type, instance) in dev_list {
            let mut dev = BytesStart::new(dev_type);
            dev.push_attribute(("instance", instance));
            writer
                .write_event(Event::Empty(dev))
                .map_err(|e| format!("device <{dev_type}>: {e}"))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("devices")))
            .map_err(|e| format!("</devices>: {e}"))?;
        writer
            .write_event(Event::End(BytesEnd::new("CustomisationUIHeader")))
            .map_err(|e| format!("</CustomisationUIHeader>: {e}"))?;

        // ── <modifiers/> (kept empty) ────────────────────────────────────────────
        writer
            .write_event(Event::Empty(BytesStart::new("modifiers")))
            .map_err(|e| format!("<modifiers>: {e}"))?;

        // ── actionmaps with actual custom binds ──────────────────────────────────
        for (map_name, action_map) in &self.action_maps {
            // Only write an actionmap if it has at least one action with active custom binds
            let custom_actions: Vec<_> = action_map
                .actions
                .values()
                .filter(|binding| {
                    binding
                        .custom_binds
                        .as_ref()
                        .is_some_and(|b| b.has_active_binds())
                })
                .collect();

            if custom_actions.is_empty() {
                continue;
            }

            let mut am = BytesStart::new("actionmap");
            am.push_attribute(("name", map_name.as_ref()));
            writer
                .write_event(Event::Start(am))
                .map_err(|e| format!("<actionmap name=\"{}\">: {e}", map_name))?;

            for action in custom_actions {
                let custom = action.custom_binds.as_ref().expect("checked above");

                let mut action_elem = BytesStart::new("action");
                action_elem.push_attribute(("name", action.action_name.as_ref()));
                writer
                    .write_event(Event::Start(action_elem))
                    .map_err(|e| format!("<action name=\"{}\">: {e}", action.action_name))?;

                // Keyboard rebinds
                for bind in &custom.keyboard {
                    if let Some(input_val) =
                        bind_to_input_with_prefix(&bind.main, &bind.modifiers, kb_inst, mo_inst)
                    {
                        let mut rebind = BytesStart::new("rebind");
                        rebind.push_attribute(("device", "keyboard"));
                        if bind.origin == BindOrigin::Generated {
                            // Generated binds default to "press" unless caller set a specific mode on the bind
                            rebind.push_attribute(("activationMode", "press"));
                        }
                        rebind.push_attribute(("input", input_val.as_str()));
                        writer
                            .write_event(Event::Empty(rebind))
                            .map_err(|e| format!("keyboard rebind: {e}"))?;
                    }
                }

                // Mouse rebinds
                for bind in &custom.mouse {
                    if let Some(input_val) =
                        bind_to_input_with_prefix(&bind.main, &bind.modifiers, kb_inst, mo_inst)
                    {
                        let mut rebind = BytesStart::new("rebind");
                        rebind.push_attribute(("device", "mouse"));
                        if bind.origin == BindOrigin::Generated {
                            rebind.push_attribute(("activationMode", "press"));
                        }
                        rebind.push_attribute(("input", input_val.as_str()));
                        writer
                            .write_event(Event::Empty(rebind))
                            .map_err(|e| format!("mouse rebind: {e}"))?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new("action")))
                    .map_err(|e| format!("</action>: {e}"))?;
            }

            writer
                .write_event(Event::End(BytesEnd::new("actionmap")))
                .map_err(|e| format!("</actionmap>: {e}"))?;
        }

        // ── </ActionMaps> ────────────────────────────────────────────────────────
        writer
            .write_event(Event::End(BytesEnd::new("ActionMaps")))
            .map_err(|e| format!("</ActionMaps>: {e}"))?;

        Ok(())
    }
}

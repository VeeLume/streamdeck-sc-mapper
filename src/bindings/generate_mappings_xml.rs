use quick_xml::events::{ BytesDecl, BytesEnd, BytesStart, Event };
use quick_xml::Writer;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::bindings::action_bindings::ActionBindings;
use crate::bindings::bind::BindOrigin;
use crate::bindings::bind_tokens::bind_to_input_with_prefix;

impl ActionBindings {
    pub fn generate_mapping_xml<P: AsRef<Path>>(
        &self,
        output_path: P,
        devices: Option<&[(&str, &str)]>,
        profile_name: &str
    ) -> Result<(), String> {
        // ---- file & writer ----
        let file = File::create(&output_path).map_err(|e| {
            format!("Failed to create XML file: {e} at {}", output_path.as_ref().display())
        })?;
        let mut writer = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

        // ---- prolog ----
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .map_err(|e| format!("XML declaration write error: {e}"))?;

        // <ActionMaps>
        let mut root = BytesStart::new("ActionMaps");
        root.push_attribute(("version", "1"));
        root.push_attribute(("optionsVersion", "2"));
        root.push_attribute(("rebindVersion", "2"));
        root.push_attribute(("profileName", profile_name));
        writer
            .write_event(Event::Start(root))
            .map_err(|e| format!("Failed to write <ActionMaps>: {e}"))?;

        // <CustomisationUIHeader>
        let mut header = BytesStart::new("CustomisationUIHeader");
        header.push_attribute(("label", profile_name));
        header.push_attribute(("description", ""));
        header.push_attribute(("image", ""));
        writer
            .write_event(Event::Start(header))
            .map_err(|e| format!("Failed to write <CustomisationUIHeader>: {e}"))?;

        // <devices>
        writer
            .write_event(Event::Start(BytesStart::new("devices")))
            .map_err(|e| format!("Failed to write <devices>: {e}"))?;

        // Default to keyboard + mouse
        let default_devices = [
            ("keyboard", "1"),
            ("mouse", "1"),
        ];
        let dev_list = devices.unwrap_or(&default_devices);
        for &(dev_type, instance) in dev_list {
            let mut dev = BytesStart::new(dev_type);
            dev.push_attribute(("instance", instance));
            writer
                .write_event(Event::Empty(dev))
                .map_err(|e| format!("Failed to write device {dev_type}: {e}"))?;
        }

        let kb_inst = devices
            .and_then(|d|
                d
                    .iter()
                    .find(|(t, _)| *t == "keyboard")
                    .map(|(_, i)| *i)
            )
            .unwrap_or("1");
        let mo_inst = devices
            .and_then(|d|
                d
                    .iter()
                    .find(|(t, _)| *t == "mouse")
                    .map(|(_, i)| *i)
            )
            .unwrap_or("1");

        writer
            .write_event(Event::End(BytesEnd::new("devices")))
            .map_err(|e| format!("Failed to write </devices>: {e}"))?;
        writer
            .write_event(Event::End(BytesEnd::new("CustomisationUIHeader")))
            .map_err(|e| format!("Failed to write </CustomisationUIHeader>: {e}"))?;

        // <modifiers/> (empty block)
        writer
            .write_event(Event::Empty(BytesStart::new("modifiers")))
            .map_err(|e| format!("Failed to write <modifiers>: {e}"))?;

        // ---- actionmaps with custom binds ----
        for (map_name, action_map) in &self.action_maps {
            // Only actions that actually have *active* custom binds get emitted
            let custom_actions: Vec<_> = action_map.actions
                .values()
                .filter(|binding| {
                    binding.custom_binds.as_ref().map_or(false, |b| b.has_active_binds())
                })
                .collect();

            if custom_actions.is_empty() {
                continue;
            }

            let mut am = BytesStart::new("actionmap");
            am.push_attribute(("name", map_name.as_ref())); // Arc<str> -> &str
            writer
                .write_event(Event::Start(am))
                .map_err(|e| format!("Failed to write <actionmap>: {e}"))?;

            for action in custom_actions {
                let custom = action.custom_binds.as_ref().unwrap();

                let mut action_elem = BytesStart::new("action");
                action_elem.push_attribute(("name", action.action_name.as_ref()));
                writer
                    .write_event(Event::Start(action_elem))
                    .map_err(|e| format!("Failed to write <action>: {e}"))?;

                // Keyboard rebinds
                for bind in &custom.keyboard {
                    if
                        let Some(input_val) = bind_to_input_with_prefix(
                            &bind.main,
                            &bind.modifiers,
                            kb_inst,
                            mo_inst
                        )
                    {
                        let mut rebind = BytesStart::new("rebind");
                        rebind.push_attribute(("device", "keyboard"));
                        if bind.origin == BindOrigin::Generated {
                            rebind.push_attribute(("activationMode", "press"));
                        }
                        rebind.push_attribute(("input", input_val.as_str()));
                        writer
                            .write_event(Event::Empty(rebind))
                            .map_err(|e| format!("Failed to write keyboard rebind: {e}"))?;
                    }
                }

                // Mouse rebinds
                for bind in &custom.mouse {
                    if
                        let Some(input_val) = bind_to_input_with_prefix(
                            &bind.main,
                            &bind.modifiers,
                            kb_inst,
                            mo_inst
                        )
                    {
                        let mut rebind = BytesStart::new("rebind");
                        rebind.push_attribute(("device", "mouse"));
                        if bind.origin == BindOrigin::Generated {
                            rebind.push_attribute(("activationMode", "press"));
                        }
                        rebind.push_attribute(("input", input_val.as_str()));
                        writer
                            .write_event(Event::Empty(rebind))
                            .map_err(|e| format!("Failed to write mouse rebind: {e}"))?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new("action")))
                    .map_err(|e| format!("Failed to write </action>: {e}"))?;
            }

            writer
                .write_event(Event::End(BytesEnd::new("actionmap")))
                .map_err(|e| format!("Failed to write </actionmap>: {e}"))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("ActionMaps")))
            .map_err(|e| format!("Failed to write </ActionMaps>: {e}"))?;

        Ok(())
    }
}

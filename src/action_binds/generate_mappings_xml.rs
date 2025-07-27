use quick_xml::Writer;
use quick_xml::events::{ BytesDecl, BytesEnd, BytesStart, Event };
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::action_binds::action_bindings::ActionBindings;

impl ActionBindings {
    pub fn generate_mapping_xml<P: AsRef<Path>>(
        &self,
        output_path: P,
        devices: Option<&[(&str, &str)]>,
        profile_name: &str
    ) -> Result<(), String> {
        let file = File::create(&output_path).map_err(|e|
            format!("Failed to create XML file: {e} at {}", output_path.as_ref().display())
        )?;
        let mut writer = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .map_err(|e| format!("XML declaration write error: {e}"))?;

        let mut root = BytesStart::new("ActionMaps");
        root.push_attribute(("version", "1"));
        root.push_attribute(("optionsVersion", "2"));
        root.push_attribute(("rebindVersion", "2"));
        root.push_attribute(("profileName", profile_name));
        writer
            .write_event(Event::Start(root))
            .map_err(|e| format!("Failed to write <ActionMaps>: {e}"))?;

        // CustomisationUIHeader
        let mut header = BytesStart::new("CustomisationUIHeader");
        header.push_attribute(("label", profile_name));
        header.push_attribute(("description", ""));
        header.push_attribute(("image", ""));
        writer
            .write_event(Event::Start(header))
            .map_err(|e| format!("Failed to write <CustomisationUIHeader>: {e}"))?;

        // Devices
        writer
            .write_event(Event::Start(BytesStart::new("devices")))
            .map_err(|e| format!("Failed to write <devices>: {e}"))?;

        let dev_list = devices.unwrap_or(&[("keyboard", "1")]);
        for &(dev_type, instance) in dev_list {
            let mut dev = BytesStart::new(dev_type);
            dev.push_attribute(("instance", instance));
            writer
                .write_event(Event::Empty(dev))
                .map_err(|e| format!("Failed to write device {dev_type}: {e}"))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("devices")))
            .map_err(|e| format!("Failed to write </devices>: {e}"))?;
        writer
            .write_event(Event::End(BytesEnd::new("CustomisationUIHeader")))
            .map_err(|e| format!("Failed to write </CustomisationUIHeader>: {e}"))?;

        // Empty modifiers block
        writer
            .write_event(Event::Empty(BytesStart::new("modifiers")))
            .map_err(|e| format!("Failed to write <modifiers>: {e}"))?;

        for (map_name, action_map) in &self.action_maps {
            let custom_actions: Vec<_> = action_map.actions
                .values()
                .filter(|binding|
                    binding.custom_binds.as_ref().map_or(false, |b| b.has_active_binds())
                )
                .collect();

            if custom_actions.is_empty() {
                continue;
            }

            let mut am = BytesStart::new("actionmap");
            am.push_attribute(("name", map_name.as_str()));
            writer
                .write_event(Event::Start(am))
                .map_err(|e| format!("Failed to write <actionmap>: {e}"))?;

            for action in custom_actions {
                let custom = action.custom_binds.as_ref().unwrap();
                let mut action_elem = BytesStart::new("action");
                action_elem.push_attribute(("name", action.action_name.as_str()));
                writer
                    .write_event(Event::Start(action_elem))
                    .map_err(|e| format!("Failed to write <action>: {e}"))?;

                for bind in &custom.keyboard {
                    let mut rebind = BytesStart::new("rebind");
                    rebind.push_attribute(("device", "keyboard"));
                    rebind.push_attribute(("activationMode", "press"));
                    rebind.push_attribute(("input", format!("kb1_{}", bind).as_str()));
                    writer
                        .write_event(Event::Empty(rebind))
                        .map_err(|e| format!("Failed to write keyboard rebind: {e}"))?;
                }

                for bind in &custom.mouse {
                    let mut rebind = BytesStart::new("rebind");
                    rebind.push_attribute(("device", "mouse"));
                    rebind.push_attribute(("activationMode", "press"));
                    rebind.push_attribute(("input", format!("mo1_{}", bind).as_str()));
                    writer
                        .write_event(Event::Empty(rebind))
                        .map_err(|e| format!("Failed to write mouse rebind: {e}"))?;
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

        self.logger.log(&format!("✅ Wrote mapping XML to {}", output_path.as_ref().display()));

        Ok(())
    }
}

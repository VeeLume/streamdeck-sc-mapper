use std::collections::{ HashMap, HashSet };
use once_cell::sync::Lazy;

use crate::action_binds::bind::Bind;

pub static SKIP_ACTION_MAPS: Lazy<HashSet<String>> = Lazy::new(|| {
    [
        "IFCS_controls",
        "debug",
        "zero_gravity_traversal",
        "hacking",
        "RemoteRigidEntityController",
        "character_customizer",
        "flycam",
        "stopwatch",
        "spaceship_auto_weapons",
        "server_renderer",
        "vehicle_mobiglas",
    ]
        .into_iter()
        .map(String::from)
        .collect()
});

pub static ACTION_MAP_UI_CATEGORIES: Lazy<HashMap<String, String>> = Lazy::new(|| {
    [
        ("mining", "@ui_CCFPS"),
        ("vehicle_mfd", "@ui_CG_MFDs"),
        ("mapui", "@ui_Map"),
        ("stopwatch", "@ui_CGStopWatch"),
        ("ui_textfield", "@uiCGUIGeneral"),
    ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
});

pub static CANDIDATE_KEYS: Lazy<HashSet<String>> = Lazy::new(|| {
    [
        "f1",
        "f2",
        "f3",
        "f4",
        "f5",
        "f6",
        "f7",
        "f8",
        "f9",
        "f10",
        "f11",
        "f12",
        "np_0",
        "np_1",
        "np_2",
        "np_3",
        "np_4",
        "np_5",
        "np_6",
        "np_7",
        "np_8",
        "np_9",
        "np_add",
        "np_subtract",
        "np_multiply",
        "np_divide",
        "np_period",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "0",
        "insert",
        "delete",
        "home",
        "end",
        "pgup",
        "pgdn",
        "u",
        "i",
        "o",
        "p",
        "j",
        "k",
        "l",
        "up",
        "down",
        "left",
        "right",
        "semicolon",
        "apostrophe",
        "comma",
        "period",
        "slash",
        "backslash",
        "minus",
        "equal",
    ]
        .into_iter()
        .map(String::from)
        .collect()
});

pub static CANDIDATE_MODIFIERS: Lazy<HashSet<String>> = Lazy::new(|| {
    ["lshift", "rshift", "lctrl", "rctrl", "lalt", "ralt"].into_iter().map(String::from).collect()
});

pub static DENY_COMBOS: Lazy<HashSet<Bind>> = Lazy::new(|| {
    [
        Bind::new("f4".to_string(), HashSet::from(["lalt".to_string()]), None),
        Bind::new("f9".to_string(), HashSet::from(["lalt".to_string()]), None),
        Bind::new(
            "f10".to_string(),
            HashSet::from(["lalt".to_string(), "lshift".to_string()]),
            None
        ),
        Bind::new("f1".to_string(), HashSet::from(["lalt".to_string()]), None),
    ]
        .into_iter()
        .collect()
});

pub static DISSALOWED_MODIFIERS_PER_CATEGORY: Lazy<HashMap<String, HashSet<String>>> = Lazy::new(
    || {
        [
            ("@ui_CCSpaceFlight", HashSet::from(["lshift", "lctrl", "rshift"])),
            ("@ui_CCFPS", HashSet::from(["lctrl", "lalt", "lshift"])),
        ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
            .collect()
    }
);

pub static DEFAULT_CATEGORY: &str = "@ui_CGUIGeneral";

pub static CATEGORY_GROUPS: Lazy<HashMap<String, HashSet<String>>> = Lazy::new(|| {
    let raw_groups: Vec<HashSet<&'static str>> = vec![
        HashSet::from([
            "@ui_CCSpaceFlight",
            "@ui_CGLightControllerDesc",
            "@ui_CCSeatGeneral",
            "@ui_CG_MFDs",
            "@ui_CGUIGeneral",
            "@ui_CGOpticalTracking",
            "@ui_CGInteraction",
        ]),
        HashSet::from([
            "@ui_CCVehicle",
            "@ui_CGLightControllerDesc",
            "@ui_CG_MFDs",
            "@ui_CGUIGeneral",
            "@ui_CGOpticalTracking",
            "@ui_CGInteraction",
        ]),
        HashSet::from([
            "@ui_CCTurrets",
            "@ui_CGUIGeneral",
            "@ui_CGOpticalTracking",
            "@ui_CGInteraction",
        ]),
        HashSet::from([
            "@ui_CCFPS",
            "@ui_CCEVA",
            "@ui_CGUIGeneral",
            "@ui_CGOpticalTracking",
            "@ui_CGInteraction",
        ]),
        HashSet::from(["@ui_Map", "@ui_CGUIGeneral"]),
        HashSet::from(["@ui_CGEASpectator", "@ui_CGUIGeneral"]),
        HashSet::from(["@ui_CCCamera", "@ui_CGUIGeneral"])
    ];

    let mut map = HashMap::new();

    for group in &raw_groups {
        for &cat in group {
            map.entry(cat.to_string())
                .or_insert_with(HashSet::new)
                .extend(group.iter().map(|s| s.to_string()));
        }
    }

    map
});

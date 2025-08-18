use std::collections::{ HashMap, HashSet };
use once_cell::sync::Lazy;
use streamdeck_lib::input::Key;

use crate::bindings::bind::{ Bind, BindMain };

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

pub static CANDIDATE_KEYS: Lazy<HashSet<Key>> = Lazy::new(|| {
    use Key::*;
    [
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        Np0,
        Np1,
        Np2,
        Np3,
        Np4,
        Np5,
        Np6,
        Np7,
        Np8,
        Np9,
        NpAdd,
        NpSubtract,
        NpMultiply,
        NpDivide,
        NpDecimal,
        D0,
        D1,
        D2,
        D3,
        D4,
        D5,
        D6,
        D7,
        D8,
        D9,
        Insert,
        Delete,
        Home,
        End,
        PageUp,
        PageDown,
        U,
        I,
        O,
        P,
        J,
        K,
        L,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        Semicolon,
        Comma,
        Period,
        Slash,
        Backslash,
        Minus,
        Equal,
    ]
        .into_iter()
        .collect()
});

pub static CANDIDATE_MODIFIERS: Lazy<HashSet<Key>> = Lazy::new(|| {
    use Key::*;
    [LShift, RShift, LCtrl, RCtrl, LAlt, RAlt].into_iter().collect()
});

pub static DENY_COMBOS: Lazy<HashSet<Bind>> = Lazy::new(|| {
    use Key::*;
    [
        Bind::new(Some(BindMain::Key(F4)), HashSet::from([LAlt]), None),
        Bind::new(Some(BindMain::Key(F9)), HashSet::from([LAlt]), None),
        Bind::new(Some(BindMain::Key(F10)), HashSet::from([LAlt, LShift]), None),
        Bind::new(Some(BindMain::Key(F1)), HashSet::from([LAlt]), None),
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

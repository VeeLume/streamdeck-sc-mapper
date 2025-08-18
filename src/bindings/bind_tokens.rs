// src/bind_tokens.rs (or any module in *your* crate)
use streamdeck_lib::input::{ Key, MouseButton };

/// Local trait so we can render external `Key` into the XML token vocabulary.
pub trait KeyTokenExt {
    fn to_token(&self) -> &'static str;
}

impl KeyTokenExt for Key {
    fn to_token(&self) -> &'static str {
        use Key::*;
        match *self {
            // letters
            A => "a",
            B => "b",
            C => "c",
            D => "d",
            E => "e",
            F => "f",
            G => "g",
            H => "h",
            I => "i",
            J => "j",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            O => "o",
            P => "p",
            Q => "q",
            R => "r",
            S => "s",
            T => "t",
            U => "u",
            V => "v",
            W => "w",
            X => "x",
            Y => "y",
            Z => "z",

            // number row
            D0 => "0",
            D1 => "1",
            D2 => "2",
            D3 => "3",
            D4 => "4",
            D5 => "5",
            D6 => "6",
            D7 => "7",
            D8 => "8",
            D9 => "9",

            // function
            F1 => "f1",
            F2 => "f2",
            F3 => "f3",
            F4 => "f4",
            F5 => "f5",
            F6 => "f6",
            F7 => "f7",
            F8 => "f8",
            F9 => "f9",
            F10 => "f10",
            F11 => "f11",
            F12 => "f12",

            // modifiers
            LShift => "lshift",
            RShift => "rshift",
            LCtrl => "lctrl",
            RCtrl => "rctrl",
            LAlt => "lalt",
            RAlt => "ralt",
            LWin => "lwin",
            RWin => "rwin",

            // symbols / misc
            Space => "space",
            Tab => "tab",
            Enter => "enter",
            Escape => "escape",
            Backspace => "backspace",
            Minus => "minus",
            Equal => "equals",
            LBracket => "lbracket",
            RBracket => "rbracket",
            Semicolon => "semicolon",
            Apostrophe => "apostrophe",
            Comma => "comma",
            Period => "period",
            Slash => "slash",
            Backslash => "backslash",
            Grave => "grave",
            CapsLock => "capslock",
            Print => "print",
            Pause => "pause",

            // navigation
            Insert => "insert",
            Delete => "delete",
            Home => "home",
            End => "end",
            PageUp => "pgup",
            PageDown => "pgdn",
            ArrowUp => "up",
            ArrowDown => "down",
            ArrowLeft => "left",
            ArrowRight => "right",

            // numpad
            Np0 => "np_0",
            Np1 => "np_1",
            Np2 => "np_2",
            Np3 => "np_3",
            Np4 => "np_4",
            Np5 => "np_5",
            Np6 => "np_6",
            Np7 => "np_7",
            Np8 => "np_8",
            Np9 => "np_9",
            NpAdd => "np_add",
            NpSubtract => "np_subtract",
            NpMultiply => "np_multiply",
            NpDivide => "np_divide",
            NpEnter => "np_enter",
            NpDecimal => "np_period",
            NpLock => "np_lock",

            Menu => "menu",

            // If you ever feed Custom into XML, pick something explicit.
            Custom { .. } => "custom",
            _ => "unknown",
        }
    }
}

/// Mouse tokens used by the XML.
pub fn mouse_to_token(btn: MouseButton) -> &'static str {
    match btn {
        MouseButton::Left => "mouse1",
        MouseButton::Right => "mouse2",
        MouseButton::Middle => "mouse3",
        MouseButton::X(1) => "mouse4",
        MouseButton::X(2) => "mouse5",
        MouseButton::X(_) => "mouse5", // clamp higher X buttons
    }
}

/// Deterministic, game-friendly mod ordering: ctrl, alt, shift, then alpha.
fn mod_bucket(tok: &str) -> u8 {
    match tok {
        "lctrl" | "rctrl" => 0,
        "lalt" | "ralt" => 1,
        "lshift" | "rshift" => 2,
        _ => 3,
    }
}

/// Build the `<rebind input="...">` token without the device prefix.
pub fn bind_to_token_no_prefix(
    main: &Option<crate::bindings::bind::BindMain>,
    mods: &std::collections::HashSet<Key>
) -> Option<String> {
    use crate::bindings::bind::BindMain::*;
    let main = main.as_ref()?;

    // mods → tokens, ordered
    let mut m: Vec<&'static str> = mods
        .iter()
        .map(|k| k.to_token())
        .collect();
    m.sort_by(|a, b| mod_bucket(a).cmp(&mod_bucket(b)).then(a.cmp(b)));

    let main_tok = match *main {
        Key(k) => k.to_token(),
        Mouse(btn) => mouse_to_token(btn),
    };

    if m.is_empty() {
        Some(main_tok.to_string())
    } else {
        let mut s = m.join("+");
        s.push('+');
        s.push_str(main_tok);
        Some(s)
    }
}

/// Full token with device prefix ("kb{inst}_" or "mo{inst}_").
pub fn bind_to_input_with_prefix(
    main: &Option<crate::bindings::bind::BindMain>,
    mods: &std::collections::HashSet<Key>,
    kb_inst: &str,
    mo_inst: &str
) -> Option<String> {
    use crate::bindings::bind::BindMain::*;
    let no_prefix = bind_to_token_no_prefix(main, mods)?;
    match main.as_ref()? {
        Key(_) => Some(format!("kb{kb_inst}_{no_prefix}")),
        Mouse(_) => Some(format!("mo{mo_inst}_{no_prefix}")),
    }
}

use serde::{ Serialize, Deserialize };
use std::fmt;
use std::{ collections::HashSet };
use std::hash::{ Hash, Hasher };

use crate::action_binds::activation_mode::ActivationMode;
use crate::action_binds::constants::CANDIDATE_MODIFIERS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bind {
    pub mainkey: String,
    pub modifiers: HashSet<String>,
    pub activation_mode: Option<ActivationMode>,
    pub is_unbound: bool,
}

#[derive(Debug, Clone)]
pub enum BindParseError {
    TooManyMainKeys {
        input: String,
        main_keys: Vec<String>,
    },
    NoInput,
}

impl PartialEq for Bind {
    fn eq(&self, other: &Self) -> bool {
        self.mainkey == other.mainkey && self.modifiers == other.modifiers
    }
}

impl Eq for Bind {}

impl Hash for Bind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mainkey.hash(state);

        // Sort modifiers before hashing to ensure stable order
        let mut sorted_mods: Vec<_> = self.modifiers.iter().collect();
        sorted_mods.sort();
        for modifier in sorted_mods {
            modifier.hash(state);
        }
    }
}

impl fmt::Display for Bind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_empty() {
            write!(f, "{}", self.mainkey)
        } else {
            let mods = self.modifiers.iter().cloned().collect::<Vec<_>>().join("+");
            write!(f, "{}+{}", mods, self.mainkey)
        }
    }
}

impl Bind {
    pub fn new(
        mainkey: String,
        modifiers: HashSet<String>,
        activation_mode: Option<ActivationMode>
    ) -> Self {
        let is_unbound = mainkey.is_empty() && modifiers.is_empty();
        Bind {
            mainkey,
            modifiers,
            activation_mode,
            is_unbound,
        }
    }

    pub fn from_string(
        input: &str,
        activation_mode: Option<ActivationMode>
    ) -> Result<Self, BindParseError> {
        if input.trim().is_empty() {
            return Ok(Bind {
                mainkey: String::new(),
                modifiers: HashSet::new(),
                activation_mode,
                is_unbound: true,
            });
        }

        let parts = input.split('_').last().unwrap_or(input);
        let segments: Vec<&str> = parts
            .split('+')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        let mut modifiers = HashSet::new();
        let mut main_keys = Vec::new();

        for part in segments {
            if CANDIDATE_MODIFIERS.contains(part) {
                modifiers.insert(part.to_string());
            } else {
                main_keys.push(part.to_string());
            }
        }

        match main_keys.len() {
            0 if modifiers.len() == 1 => {
                // Modifier-only bind
                let mainkey= match modifiers.iter().next() {
                    Some(mod_key) => mod_key.clone(),
                    None => return Err(BindParseError::NoInput),
                };
                Ok(Bind {
                    mainkey: mainkey,
                    modifiers: HashSet::new(),
                    activation_mode,
                    is_unbound: false,
                })
            }
            1 => {
                let mainkey = match main_keys.iter().next() {
                    Some(key) => key.clone(),
                    None => return Err(BindParseError::NoInput),
                };
                Ok(Bind {
                    mainkey: mainkey,
                    modifiers,
                    activation_mode,
                    is_unbound: false,
                })},
            _ =>
                Err(BindParseError::TooManyMainKeys {
                    input: input.to_string(),
                    main_keys,
                }),
        }
    }

    pub fn to_string(&self) -> String {
        let mut mods: Vec<_> = self.modifiers.iter().cloned().collect();
        mods.sort();
        if mods.is_empty() {
            self.mainkey.clone()
        } else {
            format!("{}+{}", mods.join("+"), self.mainkey)
        }
    }
}

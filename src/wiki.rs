//! Per-game Paradox modding wiki URLs.
//!
//! The game is selected at compile time via feature flags, so all constants
//! resolve to the correct wiki at zero runtime cost.

use tiger_lib::LspEntryKind;

// ─── Per-game wiki roots ──────────────────────────────────────────────────────

#[cfg(feature = "vic3")]
mod urls {
    pub const BASE:                &str = "https://vic3.paradoxwikis.com";
    pub const TRIGGERS:            &str = "https://vic3.paradoxwikis.com/Trigger";
    pub const EFFECTS:             &str = "https://vic3.paradoxwikis.com/Effect";
    pub const SCOPES:              &str = "https://vic3.paradoxwikis.com/Scope";
    pub const SCRIPTED_TRIGGERS:   &str = "https://vic3.paradoxwikis.com/Scripted_trigger";
    pub const SCRIPTED_EFFECTS:    &str = "https://vic3.paradoxwikis.com/Scripted_effect";
    pub const SCRIPTED_MODIFIERS:  &str = "https://vic3.paradoxwikis.com/Modifier_modding";
    pub const EVENTS:              &str = "https://vic3.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://vic3.paradoxwikis.com/Variable";
    pub const MODDING_INDEX:       &str = "https://vic3.paradoxwikis.com/Category:Modding";
}

#[cfg(feature = "ck3")]
mod urls {
    pub const BASE:                &str = "https://ck3.paradoxwikis.com";
    pub const TRIGGERS:            &str = "https://ck3.paradoxwikis.com/Triggers";
    pub const EFFECTS:             &str = "https://ck3.paradoxwikis.com/Effects";
    pub const SCOPES:              &str = "https://ck3.paradoxwikis.com/Scopes";
    pub const SCRIPTED_TRIGGERS:   &str = "https://ck3.paradoxwikis.com/Triggers";
    pub const SCRIPTED_EFFECTS:    &str = "https://ck3.paradoxwikis.com/Scripted_effects";
    pub const SCRIPTED_MODIFIERS:  &str = "https://ck3.paradoxwikis.com/Modifiers";
    pub const EVENTS:              &str = "https://ck3.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://ck3.paradoxwikis.com/Variables";
    pub const MODDING_INDEX:       &str = "https://ck3.paradoxwikis.com/Category:Modding";
}

#[cfg(feature = "imperator")]
mod urls {
    pub const BASE:                &str = "https://imperator.paradoxwikis.com";
    pub const TRIGGERS:            &str = "https://imperator.paradoxwikis.com/Triggers";
    pub const EFFECTS:             &str = "https://imperator.paradoxwikis.com/Effects";
    pub const SCOPES:              &str = "https://imperator.paradoxwikis.com/Scopes";
    pub const SCRIPTED_TRIGGERS:   &str = "https://imperator.paradoxwikis.com/Triggers";
    pub const SCRIPTED_EFFECTS:    &str = "https://imperator.paradoxwikis.com/Effects";
    pub const SCRIPTED_MODIFIERS:  &str = "https://imperator.paradoxwikis.com/Script_Modifiers";
    pub const EVENTS:              &str = "https://imperator.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://imperator.paradoxwikis.com/Variables";
    pub const MODDING_INDEX:       &str = "https://imperator.paradoxwikis.com/Category:Modding";
}

#[cfg(feature = "hoi4")]
mod urls {
    pub const BASE:                &str = "https://hoi4.paradoxwikis.com";
    pub const TRIGGERS:            &str = "https://hoi4.paradoxwikis.com/Triggers";
    pub const EFFECTS:             &str = "https://hoi4.paradoxwikis.com/Effect";
    pub const SCOPES:              &str = "https://hoi4.paradoxwikis.com/Scopes";
    pub const SCRIPTED_TRIGGERS:   &str = "https://hoi4.paradoxwikis.com/Triggers";
    pub const SCRIPTED_EFFECTS:    &str = "https://hoi4.paradoxwikis.com/Effect";
    pub const SCRIPTED_MODIFIERS:  &str = "https://hoi4.paradoxwikis.com/Modifiers";
    pub const EVENTS:              &str = "https://hoi4.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://hoi4.paradoxwikis.com/Variables";
    pub const MODDING_INDEX:       &str = "https://hoi4.paradoxwikis.com/Category:Modding";
}

#[cfg(feature = "eu5")]
mod urls {
    pub const BASE:                &str = "https://eu5.paradoxwikis.com";
    pub const TRIGGERS:            &str = "https://eu5.paradoxwikis.com/Trigger";
    pub const EFFECTS:             &str = "https://eu5.paradoxwikis.com/Effect";
    pub const SCOPES:              &str = "https://eu5.paradoxwikis.com/Scope";
    pub const SCRIPTED_TRIGGERS:   &str = "https://eu5.paradoxwikis.com/Trigger";
    pub const SCRIPTED_EFFECTS:    &str = "https://eu5.paradoxwikis.com/Effect";
    pub const SCRIPTED_MODIFIERS:  &str = "https://eu5.paradoxwikis.com/Modifier_modding";
    pub const EVENTS:              &str = "https://eu5.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://eu5.paradoxwikis.com/Variable";
    pub const MODDING_INDEX:       &str = "https://eu5.paradoxwikis.com/Category:Modding";
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Wiki URL for an engine built-in, by kind.
pub fn builtin_wiki_url(kind: &LspEntryKind) -> &'static str {
    match kind {
        LspEntryKind::Trigger  => urls::TRIGGERS,
        LspEntryKind::Effect   => urls::EFFECTS,
        LspEntryKind::Iterator => urls::SCOPES,
    }
}

/// Wiki URL for a mod-defined scripted item, by detail string.
pub fn scripted_wiki_url(detail: &str) -> &'static str {
    match detail {
        "scripted_effect"   => urls::SCRIPTED_EFFECTS,
        "scripted_trigger"  => urls::SCRIPTED_TRIGGERS,
        "scripted_modifier" => urls::SCRIPTED_MODIFIERS,
        "event"             => urls::EVENTS,
        _                   => urls::MODDING_INDEX,
    }
}

pub fn variables_url()    -> &'static str { urls::VARIABLES }
pub fn modding_index_url() -> &'static str { urls::MODDING_INDEX }
pub fn wiki_base()         -> &'static str { urls::BASE }

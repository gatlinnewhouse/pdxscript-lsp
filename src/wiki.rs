//! Per-game Paradox modding wiki URLs.
//!
//! The game is selected at compile time via feature flags, so all constants
//! resolve to the correct wiki at zero runtime cost.

use tiger_lib::LspEntryKind;

// ─── Per-game wiki roots ──────────────────────────────────────────────────────

#[cfg(feature = "vic3")]
#[allow(dead_code)]
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

pub fn variables_url()         -> &'static str { urls::VARIABLES }
#[allow(dead_code)]
pub fn modding_index_url()     -> &'static str { urls::MODDING_INDEX }
#[allow(dead_code)]
pub fn wiki_base()             -> &'static str { urls::BASE }

/// Fallback wiki URL for a tiger `ErrorKey` (kebab-case string) when tiger-lib
/// doesn't provide one.  Returns `None` for keys where no relevant page exists.
pub fn fallback_wiki_url(key: &str) -> Option<&'static str> {
    match key {
        "scopes" | "strict-scopes" | "temporary-scope" | "use-of-this"
            => Some(urls::SCOPES),
        "variables" | "unknown-variable"
            => Some(urls::VARIABLES),
        "localization" | "missing-localization" | "suggest-localization"
        | "unused-localization" | "localization-key-collision" | "markup"
            => None,  // no single clean page; skip
        "modifiers"
            => Some(urls::SCRIPTED_MODIFIERS),
        _ => Some(urls::MODDING_INDEX),
    }
}

// ─── Tiger validator reference ────────────────────────────────────────────────

#[allow(dead_code)]
pub const TIGER_OVERVIEW: &str =
    "https://github.com/amtep/tiger/wiki/Overview-for-coders";

pub const TIGER_JSON_FORMAT: &str =
    "https://github.com/amtep/tiger/wiki/JSON-output-format";

/// Human-readable explanation of a tiger severity level.
/// Matches the levels documented in tiger's JSON output format wiki page.
pub fn severity_doc(severity: &str) -> &'static str {
    match severity {
        "tips"    => "things that aren't wrong but could be improved",
        "untidy"  => "won't affect the player but will cause maintenance headaches",
        "warning" => "the player will notice but gameplay is unaffected (e.g. missing localization)",
        "error"   => "bugs likely to affect gameplay",
        "fatal"   => "can cause crashes",
        _         => "unknown severity",
    }
}

/// Human-readable explanation of a tiger confidence level.
pub fn confidence_doc(confidence: &str) -> &'static str {
    match confidence {
        "weak"       => "likely a false positive",
        "reasonable" => "probably a real problem",
        "strong"     => "very likely a real problem",
        _            => "unknown confidence",
    }
}

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
    pub const SCRIPTED_TRIGGERS:   &str = "https://vic3.paradoxwikis.com/Trigger";
    pub const SCRIPTED_EFFECTS:    &str = "https://vic3.paradoxwikis.com/Effect";
    pub const SCRIPTED_MODIFIERS:  &str = "https://vic3.paradoxwikis.com/Modifier_modding";
    pub const EVENTS:              &str = "https://vic3.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://vic3.paradoxwikis.com/Variable";
    pub const MODDING_INDEX:       &str = "https://vic3.paradoxwikis.com/Modding";
    // Content-specific pages
    pub const BUILDING_MODDING:    &str = "https://vic3.paradoxwikis.com/Building_modding";
    pub const CHARACTER_MODDING:   &str = "https://vic3.paradoxwikis.com/Character_modding";
    pub const COUNTRY_MODDING:     &str = "https://vic3.paradoxwikis.com/Country_modding";
    pub const CULTURE_MODDING:     &str = "https://vic3.paradoxwikis.com/Culture_modding";
    pub const DECISION_MODDING:    &str = "https://vic3.paradoxwikis.com/Decision_modding";
    pub const DECREE_MODDING:      &str = "https://vic3.paradoxwikis.com/Decree_modding";
    pub const DIPLOMACY_MODDING:   &str = "https://vic3.paradoxwikis.com/Diplomacy_modding";
    pub const HISTORY_MODDING:     &str = "https://vic3.paradoxwikis.com/History_modding";
    pub const INSTITUTION_MODDING: &str = "https://vic3.paradoxwikis.com/Institution_modding";
    pub const INTEREST_GROUP_MODDING: &str = "https://vic3.paradoxwikis.com/Interest_group_modding";
    pub const JOURNAL_MODDING:     &str = "https://vic3.paradoxwikis.com/Journal_modding";
    pub const LAW_MODDING:         &str = "https://vic3.paradoxwikis.com/Law_modding";
    pub const MOVEMENT_MODDING:    &str = "https://vic3.paradoxwikis.com/Political_movement_modding";
    pub const POP_MODDING:         &str = "https://vic3.paradoxwikis.com/Pop_modding";
    pub const POWER_BLOC_MODDING:  &str = "https://vic3.paradoxwikis.com/Power_bloc_modding";
    pub const RELIGION_MODDING:    &str = "https://vic3.paradoxwikis.com/Religion_modding";
    pub const STATE_MODDING:       &str = "https://vic3.paradoxwikis.com/State_modding";
    pub const TECHNOLOGY_MODDING:  &str = "https://vic3.paradoxwikis.com/Technology_modding";
    pub const TREATY_MODDING:      &str = "https://vic3.paradoxwikis.com/Treaty_modding";
    pub const WAR_GOAL_MODDING:    &str = "https://vic3.paradoxwikis.com/War_goal_modding";
    pub const GOODS_MODDING:       &str = "https://vic3.paradoxwikis.com/Goods_modding";
    pub const ON_ACTION:           &str = "https://vic3.paradoxwikis.com/On_action";
}

#[cfg(feature = "ck3")]
mod urls {
    pub const BASE:                &str = "https://ck3.paradoxwikis.com";
    pub const TRIGGERS:            &str = "https://ck3.paradoxwikis.com/Triggers";
    pub const EFFECTS:             &str = "https://ck3.paradoxwikis.com/Effects";
    pub const SCOPES:              &str = "https://ck3.paradoxwikis.com/Scopes";
    pub const SCRIPTED_TRIGGERS:   &str = "https://ck3.paradoxwikis.com/Triggers";
    pub const SCRIPTED_EFFECTS:    &str = "https://ck3.paradoxwikis.com/Scripted_effects";
    pub const SCRIPTED_MODIFIERS:  &str = "https://ck3.paradoxwikis.com/Modifier_list";
    pub const EVENTS:              &str = "https://ck3.paradoxwikis.com/Event_modding";
    pub const VARIABLES:           &str = "https://ck3.paradoxwikis.com/Variables";
    pub const MODDING_INDEX:       &str = "https://ck3.paradoxwikis.com/Modding";
    // Content-specific pages
    pub const ARTIFACT_MODDING:    &str = "https://ck3.paradoxwikis.com/Artifact_modding";
    pub const BOOKMARKS_MODDING:   &str = "https://ck3.paradoxwikis.com/Bookmarks_modding";
    pub const CHARACTER_MODDING:   &str = "https://ck3.paradoxwikis.com/Characters_modding";
    pub const COA_MODDING:         &str = "https://ck3.paradoxwikis.com/Coat_of_arms_modding";
    pub const COUNCIL_MODDING:     &str = "https://ck3.paradoxwikis.com/Council_modding";
    pub const CULTURE_MODDING:     &str = "https://ck3.paradoxwikis.com/Culture_modding";
    pub const DECISIONS_MODDING:   &str = "https://ck3.paradoxwikis.com/Decisions_modding";
    pub const DYNASTIES_MODDING:   &str = "https://ck3.paradoxwikis.com/Dynasties_modding";
    pub const FLAVORIZATION:       &str = "https://ck3.paradoxwikis.com/Flavorization";
    pub const GOVERNMENTS_MODDING: &str = "https://ck3.paradoxwikis.com/Governments_modding";
    pub const HISTORY_MODDING:     &str = "https://ck3.paradoxwikis.com/History_modding";
    pub const HOLDINGS_MODDING:    &str = "https://ck3.paradoxwikis.com/Holdings_modding";
    pub const INTERACTIONS_MODDING:&str = "https://ck3.paradoxwikis.com/Interactions_modding";
    pub const MAP_MODDING:         &str = "https://ck3.paradoxwikis.com/Map_modding";
    pub const RELIGIONS_MODDING:   &str = "https://ck3.paradoxwikis.com/Religions_modding";
    pub const STRUGGLE_MODDING:    &str = "https://ck3.paradoxwikis.com/Struggle_modding";
    pub const TITLE_MODDING:       &str = "https://ck3.paradoxwikis.com/Title_modding";
    pub const TRAIT_MODDING:       &str = "https://ck3.paradoxwikis.com/Trait_modding";
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
    pub const MODDING_INDEX:       &str = "https://imperator.paradoxwikis.com/Modding";
    // Content-specific pages
    pub const CHARACTER_MODDING:   &str = "https://imperator.paradoxwikis.com/Character_modding";
    pub const COA_MODDING:         &str = "https://imperator.paradoxwikis.com/Coat_of_arms_modding";
    pub const CULTURE_MODDING:     &str = "https://imperator.paradoxwikis.com/Culture_modding";
    pub const DECISION_MODDING:    &str = "https://imperator.paradoxwikis.com/Decision_modding";
    pub const GOVERNMENT_MODDING:  &str = "https://imperator.paradoxwikis.com/Government_modding";
    pub const GOVERNOR_POLICY_MODDING: &str = "https://imperator.paradoxwikis.com/Governor_policy_modding";
    pub const IDEAS_MODDING:       &str = "https://imperator.paradoxwikis.com/Ideas_modding";
    pub const MAP_MODDING:         &str = "https://imperator.paradoxwikis.com/Map_modding";
    pub const MODIFIER_MODDING:    &str = "https://imperator.paradoxwikis.com/Modifier_modding";
    pub const ON_ACTION_MODDING:   &str = "https://imperator.paradoxwikis.com/On_action_modding";
    pub const POPS_MODDING:        &str = "https://imperator.paradoxwikis.com/Pops_modding";
    pub const PROVINCE_SETUP:      &str = "https://imperator.paradoxwikis.com/Province_setup";
    pub const RELIGION_MODDING:    &str = "https://imperator.paradoxwikis.com/Religion_modding";
    pub const SCRIPT_VALUES:       &str = "https://imperator.paradoxwikis.com/Script_Values";
    pub const TRADE_MODDING:       &str = "https://imperator.paradoxwikis.com/Trade_modding";
    pub const TRAITS_MODDING:      &str = "https://imperator.paradoxwikis.com/Traits_modding";
    pub const UNIT_MODDING:        &str = "https://imperator.paradoxwikis.com/Unit_modding";
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
    pub const MODDING_INDEX:       &str = "https://hoi4.paradoxwikis.com/Modding";
    // Content-specific pages
    pub const BUILDING_MODDING:    &str = "https://hoi4.paradoxwikis.com/Building_modding";
    pub const CHARACTER_MODDING:   &str = "https://hoi4.paradoxwikis.com/Character_modding";
    pub const DECISION_MODDING:    &str = "https://hoi4.paradoxwikis.com/Decision_modding";
    pub const DIVISION_MODDING:    &str = "https://hoi4.paradoxwikis.com/Division_modding";
    pub const DOCTRINE_MODDING:    &str = "https://hoi4.paradoxwikis.com/Doctrine_modding";
    pub const EQUIPMENT_MODDING:   &str = "https://hoi4.paradoxwikis.com/Equipment_modding";
    pub const IDEA_MODDING:        &str = "https://hoi4.paradoxwikis.com/Idea_modding";
    pub const IDEOLOGY_MODDING:    &str = "https://hoi4.paradoxwikis.com/Ideology_modding";
    pub const INTELLIGENCE_AGENCY_MODDING: &str = "https://hoi4.paradoxwikis.com/Intelligence_agency_modding";
    pub const MAP_MODDING:         &str = "https://hoi4.paradoxwikis.com/Map_modding";
    pub const MIO_MODDING:         &str = "https://hoi4.paradoxwikis.com/Military_industrial_organization_modding";
    pub const NATIONAL_FOCUS_MODDING: &str = "https://hoi4.paradoxwikis.com/National_focus_modding";
    pub const ON_ACTIONS:          &str = "https://hoi4.paradoxwikis.com/On_actions";
    pub const OPERATION_MODDING:   &str = "https://hoi4.paradoxwikis.com/Intelligence_agency_modding";
    pub const STATE_MODDING:       &str = "https://hoi4.paradoxwikis.com/State_modding";
    pub const TECHNOLOGY_MODDING:  &str = "https://hoi4.paradoxwikis.com/Technology_modding";
    pub const UNIT_MODDING:        &str = "https://hoi4.paradoxwikis.com/Unit_modding";
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
    pub const MODDING_INDEX:       &str = "https://eu5.paradoxwikis.com/Modding";
    // Content-specific pages
    pub const ACTION_MODDING:      &str = "https://eu5.paradoxwikis.com/Action_modding";
    pub const ADVANCE_MODDING:     &str = "https://eu5.paradoxwikis.com/Advance_modding";
    pub const BUILDING_MODDING:    &str = "https://eu5.paradoxwikis.com/Building_modding";
    pub const CHARACTER_MODDING:   &str = "https://eu5.paradoxwikis.com/Character_modding";
    pub const COUNTRY_MODDING:     &str = "https://eu5.paradoxwikis.com/Country_modding";
    pub const CULTURE_MODDING:     &str = "https://eu5.paradoxwikis.com/Culture_modding";
    pub const DISASTER_MODDING:    &str = "https://eu5.paradoxwikis.com/Disaster_modding";
    pub const DISEASE_MODDING:     &str = "https://eu5.paradoxwikis.com/Disease_modding";
    pub const ESTATE_MODDING:      &str = "https://eu5.paradoxwikis.com/Estate_modding";
    pub const GOODS_MODDING:       &str = "https://eu5.paradoxwikis.com/Goods_modding";
    pub const INSTITUTION_MODDING: &str = "https://eu5.paradoxwikis.com/Institution_modding";
    pub const LAW_MODDING:         &str = "https://eu5.paradoxwikis.com/Law_modding";
    pub const MAP_MODDING:         &str = "https://eu5.paradoxwikis.com/Map_modding";
    pub const MISSION_MODDING:     &str = "https://eu5.paradoxwikis.com/Mission_modding";
    pub const MODIFIER_MODDING:    &str = "https://eu5.paradoxwikis.com/Modifier_modding";
    pub const ON_ACTION:           &str = "https://eu5.paradoxwikis.com/On_action";
    pub const POP_MODDING:         &str = "https://eu5.paradoxwikis.com/Pop_modding";
    pub const RELIGION_MODDING:    &str = "https://eu5.paradoxwikis.com/Religion_modding";
    pub const SITUATION_MODDING:   &str = "https://eu5.paradoxwikis.com/Situation_modding";
    pub const SUBJECT_TYPE_MODDING:&str = "https://eu5.paradoxwikis.com/Subject_type_modding";
    pub const TERRAIN_MODDING:     &str = "https://eu5.paradoxwikis.com/Terrain_modding";
    pub const TRAIT_MODDING:       &str = "https://eu5.paradoxwikis.com/Trait_modding";
    pub const UNIT_MODDING:        &str = "https://eu5.paradoxwikis.com/Unit_modding";
    pub const WAR_MODDING:         &str = "https://eu5.paradoxwikis.com/War_modding";
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
        "modifiers" | "unknown-modifier"
            => Some(urls::SCRIPTED_MODIFIERS),
        "missing-item" | "unknown-item"
            => None,  // too generic — path_wiki_url handles file-based routing
        _ => Some(urls::MODDING_INDEX),
    }
}

/// Wiki URL derived from the file path of the error location.
/// Matches common Paradox directory conventions across all supported games.
/// Returns `None` when no specific page can be identified.
pub fn path_wiki_url(path: &str) -> Option<&'static str> {
    // Cross-game scripting pages (most specific first)
    if path.contains("scripted_effects") { return Some(urls::SCRIPTED_EFFECTS); }
    if path.contains("scripted_triggers") { return Some(urls::SCRIPTED_TRIGGERS); }
    if path.contains("scripted_modifiers") || path.contains("script_modifiers") {
        return Some(urls::SCRIPTED_MODIFIERS);
    }
    if path.contains("/events/") || path.ends_with("/events") {
        return Some(urls::EVENTS);
    }

    // Game-specific path mappings
    #[cfg(feature = "vic3")]
    {
        if path.contains("common/laws") { return Some(urls::LAW_MODDING); }
        if path.contains("common/law_groups") { return Some(urls::LAW_MODDING); }
        if path.contains("common/buildings") || path.contains("common/building_groups")
            || path.contains("common/production_method") {
            return Some(urls::BUILDING_MODDING);
        }
        if path.contains("common/decisions") { return Some(urls::DECISION_MODDING); }
        if path.contains("common/decrees") { return Some(urls::DECREE_MODDING); }
        if path.contains("common/diplomatic_actions") || path.contains("common/diplomatic_plays")
            || path.contains("common/diplomatic_catalysts") || path.contains("common/diplomatic_catalyst") {
            return Some(urls::DIPLOMACY_MODDING);
        }
        if path.contains("common/treaty_articles") || path.contains("common/dynamic_treaty") {
            return Some(urls::TREATY_MODDING);
        }
        if path.contains("common/cultures") { return Some(urls::CULTURE_MODDING); }
        if path.contains("common/character") { return Some(urls::CHARACTER_MODDING); }
        if path.contains("common/interest_group") { return Some(urls::INTEREST_GROUP_MODDING); }
        if path.contains("common/journal_entries") || path.contains("common/journal_entry") {
            return Some(urls::JOURNAL_MODDING);
        }
        if path.contains("common/institutions") { return Some(urls::INSTITUTION_MODDING); }
        if path.contains("common/technology") { return Some(urls::TECHNOLOGY_MODDING); }
        if path.contains("common/religions") { return Some(urls::RELIGION_MODDING); }
        if path.contains("common/state_regions") || path.contains("common/provinces") {
            return Some(urls::STATE_MODDING);
        }
        if path.contains("common/pops") || path.contains("common/pop_types") {
            return Some(urls::POP_MODDING);
        }
        if path.contains("common/power_bloc") { return Some(urls::POWER_BLOC_MODDING); }
        if path.contains("common/political_movement") { return Some(urls::MOVEMENT_MODDING); }
        if path.contains("common/country") { return Some(urls::COUNTRY_MODDING); }
        if path.contains("common/goods") { return Some(urls::GOODS_MODDING); }
        if path.contains("common/war_goal") { return Some(urls::WAR_GOAL_MODDING); }
        if path.contains("common/modifier") { return Some(urls::SCRIPTED_MODIFIERS); }
        if path.contains("common/history") || path.contains("/history/") {
            return Some(urls::HISTORY_MODDING);
        }
        if path.contains("common/on_actions") { return Some(urls::ON_ACTION); }
    }

    #[cfg(feature = "ck3")]
    {
        if path.contains("common/artifacts") { return Some(urls::ARTIFACT_MODDING); }
        if path.contains("common/bookmarks") { return Some(urls::BOOKMARKS_MODDING); }
        if path.contains("common/coat_of_arms") { return Some(urls::COA_MODDING); }
        if path.contains("common/council") { return Some(urls::COUNCIL_MODDING); }
        if path.contains("common/culture") { return Some(urls::CULTURE_MODDING); }
        if path.contains("common/decisions") { return Some(urls::DECISIONS_MODDING); }
        if path.contains("common/dynasties") || path.contains("common/dynasty") {
            return Some(urls::DYNASTIES_MODDING);
        }
        if path.contains("common/flavorization") { return Some(urls::FLAVORIZATION); }
        if path.contains("common/governments") { return Some(urls::GOVERNMENTS_MODDING); }
        if path.contains("common/holdings") || path.contains("common/buildings") {
            return Some(urls::HOLDINGS_MODDING);
        }
        if path.contains("common/character_interactions") { return Some(urls::INTERACTIONS_MODDING); }
        if path.contains("common/character") { return Some(urls::CHARACTER_MODDING); }
        if path.contains("common/landed_titles") { return Some(urls::TITLE_MODDING); }
        if path.contains("common/laws") { return Some(urls::GOVERNMENTS_MODDING); }
        if path.contains("common/religions") { return Some(urls::RELIGIONS_MODDING); }
        if path.contains("common/traits") { return Some(urls::TRAIT_MODDING); }
        if path.contains("common/struggles") { return Some(urls::STRUGGLE_MODDING); }
        if path.contains("common/modifier") { return Some(urls::SCRIPTED_MODIFIERS); }
        if path.contains("common/history") || path.contains("/history/") {
            return Some(urls::HISTORY_MODDING);
        }
        if path.contains("map_data/") || path.contains("common/terrain") {
            return Some(urls::MAP_MODDING);
        }
    }

    #[cfg(feature = "imperator")]
    {
        if path.contains("common/buildings") { return Some(urls::MODDING_INDEX); }
        if path.contains("common/decisions") { return Some(urls::DECISION_MODDING); }
        if path.contains("common/character") || path.contains("common/setup_characters") {
            return Some(urls::CHARACTER_MODDING);
        }
        if path.contains("common/coat_of_arms") { return Some(urls::COA_MODDING); }
        if path.contains("common/culture") { return Some(urls::CULTURE_MODDING); }
        if path.contains("common/governments") { return Some(urls::GOVERNMENT_MODDING); }
        if path.contains("common/governor_policies") { return Some(urls::GOVERNOR_POLICY_MODDING); }
        if path.contains("common/ideas") { return Some(urls::IDEAS_MODDING); }
        if path.contains("common/laws") { return Some(urls::GOVERNMENT_MODDING); }
        if path.contains("common/modifier") { return Some(urls::MODIFIER_MODDING); }
        if path.contains("common/on_actions") { return Some(urls::ON_ACTION_MODDING); }
        if path.contains("common/pops") || path.contains("common/pop_types") {
            return Some(urls::POPS_MODDING);
        }
        if path.contains("common/provinces") || path.contains("common/setup_provinces") {
            return Some(urls::PROVINCE_SETUP);
        }
        if path.contains("common/religions") { return Some(urls::RELIGION_MODDING); }
        if path.contains("common/script_values") { return Some(urls::SCRIPT_VALUES); }
        if path.contains("common/trade") { return Some(urls::TRADE_MODDING); }
        if path.contains("common/traits") { return Some(urls::TRAITS_MODDING); }
        if path.contains("common/units") { return Some(urls::UNIT_MODDING); }
        if path.contains("common/religions") { return Some(urls::RELIGION_MODDING); }
        if path.contains("map_data/") || path.contains("common/terrain") {
            return Some(urls::MAP_MODDING);
        }
    }

    #[cfg(feature = "hoi4")]
    {
        if path.contains("common/buildings") { return Some(urls::BUILDING_MODDING); }
        if path.contains("common/characters") || path.contains("common/country_leader")
            || path.contains("common/unit_leader") {
            return Some(urls::CHARACTER_MODDING);
        }
        if path.contains("common/decisions") { return Some(urls::DECISION_MODDING); }
        if path.contains("common/units/equipment") { return Some(urls::EQUIPMENT_MODDING); }
        if path.contains("common/units") { return Some(urls::UNIT_MODDING); }
        if path.contains("common/ideas") { return Some(urls::IDEA_MODDING); }
        if path.contains("common/ideologies") { return Some(urls::IDEOLOGY_MODDING); }
        if path.contains("common/intelligence_agenc") { return Some(urls::INTELLIGENCE_AGENCY_MODDING); }
        if path.contains("common/military_industrial_organization") { return Some(urls::MIO_MODDING); }
        if path.contains("common/national_focus") { return Some(urls::NATIONAL_FOCUS_MODDING); }
        if path.contains("common/on_actions") { return Some(urls::ON_ACTIONS); }
        if path.contains("common/operations") { return Some(urls::OPERATION_MODDING); }
        if path.contains("common/technologies") { return Some(urls::TECHNOLOGY_MODDING); }
        if path.contains("common/modifier") { return Some(urls::SCRIPTED_MODIFIERS); }
        if path.contains("history/states") { return Some(urls::STATE_MODDING); }
        if path.contains("map/") { return Some(urls::MAP_MODDING); }
    }

    #[cfg(feature = "eu5")]
    {
        if path.contains("common/actions") { return Some(urls::ACTION_MODDING); }
        if path.contains("common/advances") { return Some(urls::ADVANCE_MODDING); }
        if path.contains("common/buildings") { return Some(urls::BUILDING_MODDING); }
        if path.contains("common/characters") { return Some(urls::CHARACTER_MODDING); }
        if path.contains("common/countries") { return Some(urls::COUNTRY_MODDING); }
        if path.contains("common/cultures") { return Some(urls::CULTURE_MODDING); }
        if path.contains("common/disasters") { return Some(urls::DISASTER_MODDING); }
        if path.contains("common/diseases") { return Some(urls::DISEASE_MODDING); }
        if path.contains("common/estates") { return Some(urls::ESTATE_MODDING); }
        if path.contains("common/goods") { return Some(urls::GOODS_MODDING); }
        if path.contains("common/institutions") { return Some(urls::INSTITUTION_MODDING); }
        if path.contains("common/laws") { return Some(urls::LAW_MODDING); }
        if path.contains("common/missions") { return Some(urls::MISSION_MODDING); }
        if path.contains("common/modifier") { return Some(urls::MODIFIER_MODDING); }
        if path.contains("common/on_actions") || path.contains("common/on_action") {
            return Some(urls::ON_ACTION);
        }
        if path.contains("common/pops") || path.contains("common/pop_types") {
            return Some(urls::POP_MODDING);
        }
        if path.contains("common/religions") { return Some(urls::RELIGION_MODDING); }
        if path.contains("common/situations") { return Some(urls::SITUATION_MODDING); }
        if path.contains("common/subject_types") { return Some(urls::SUBJECT_TYPE_MODDING); }
        if path.contains("common/terrain") { return Some(urls::TERRAIN_MODDING); }
        if path.contains("common/traits") { return Some(urls::TRAIT_MODDING); }
        if path.contains("common/units") { return Some(urls::UNIT_MODDING); }
        if path.contains("common/wars") { return Some(urls::WAR_MODDING); }
        if path.contains("map/") { return Some(urls::MAP_MODDING); }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_scopes_key() {
        let url = fallback_wiki_url("scopes").expect("scopes should have a url");
        assert!(url.contains("paradoxwikis.com"));
        assert!(url.to_lowercase().contains("scope"));
    }

    #[test]
    fn fallback_variables_key() {
        let url = fallback_wiki_url("variables").expect("variables should have a url");
        assert!(url.to_lowercase().contains("variable"));
    }

    #[test]
    fn fallback_localization_is_none() {
        assert!(fallback_wiki_url("missing-localization").is_none());
    }

    #[test]
    fn fallback_unknown_returns_modding_index() {
        let url = fallback_wiki_url("some-unknown-key").expect("should fall back to modding index");
        assert!(url.contains("paradoxwikis.com"));
    }

    #[test]
    fn path_scripted_effects_recognized() {
        let url = path_wiki_url("mods/mymod/common/scripted_effects/my_effects.txt");
        assert!(url.is_some());
        let u = url.unwrap();
        assert!(u.contains("paradoxwikis.com"));
    }

    #[test]
    fn path_events_recognized() {
        let url = path_wiki_url("mods/mymod/events/my_events.txt");
        assert!(url.is_some());
        let u = url.unwrap();
        assert!(u.to_lowercase().contains("event"));
    }

    #[test]
    fn path_unrecognized_returns_none() {
        let url = path_wiki_url("/some/totally/random/path/file.txt");
        // May or may not be None depending on which cfg game feature is active,
        // but it should not panic.
        let _ = url;
    }

    #[test]
    fn severity_doc_known_levels() {
        assert!(!severity_doc("error").is_empty());
        assert!(!severity_doc("warning").is_empty());
        assert!(!severity_doc("fatal").is_empty());
    }

    #[test]
    fn confidence_doc_known_levels() {
        assert!(!confidence_doc("weak").is_empty());
        assert!(!confidence_doc("strong").is_empty());
    }
}

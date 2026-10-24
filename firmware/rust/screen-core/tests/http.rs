//! Contrat HTTP : validation atomique, refus pendant les cycles et actions.
mod common;

use common::Rig;
use screen_core::config::MemoryConfigStore;
use screen_core::core::{Action, ActionCommand, ActionSource, Clock, Core};
use screen_core::http::*;
use screen_core::json::{self, Json};

fn post_action(rig: &mut Rig, body: &[u8]) -> HttpResponse {
    let clock = rig.clock();
    handle_post_action(&mut rig.core, clock, body)
}

fn post_config(rig: &mut Rig, body: &[u8]) -> HttpResponse {
    let clock = rig.clock();
    handle_post_config(&mut rig.core, clock, body)
}

#[test]
fn get_defaults_has_the_schema_11_shape() {
    let core = Core::new(MemoryConfigStore::default(), Clock::new(1_000_000));
    let response = handle_get_config(&core);
    assert_eq!(response.status, "200 OK");
    let root = json::parse(response.body.as_bytes()).unwrap();
    assert_eq!(root.get("version").and_then(Json::as_f64), Some(11.0));
    assert_eq!(
        root.get("active_profile").and_then(Json::as_str),
        Some("single espresso")
    );
    let Some(Json::Array(profiles)) = root.get("profiles") else {
        panic!("profiles");
    };
    assert_eq!(profiles.len(), 1);
}

#[test]
fn config_patch_is_atomic_and_persisted() {
    let mut rig = Rig::new();
    let before = rig.core.config();
    let writes = rig.core.store.writes;
    let response = post_config(
        &mut rig,
        br#"{"version":11,"brew":{"target_weight_g":42.5},"unknown":0}"#,
    );
    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(rig.core.config(), before);
    assert_eq!(rig.core.store.writes, writes);
    let response = post_config(
        &mut rig,
        br#"{"version":11,"brew":{"target_weight_g":42.5}}"#,
    );
    assert_eq!(response.status, "200 OK");
    assert_eq!(rig.core.config().target_weight_g, 42.5);
    assert_eq!(rig.core.store.writes, writes + 1);
    assert_eq!(response.body, handle_get_config(&rig.core).body);
    rig.core.store.fail_writes = true;
    let before = rig.core.config();
    let response = post_config(
        &mut rig,
        br#"{"version":11,"brew":{"target_weight_g":43.0}}"#,
    );
    assert_eq!(response.status, "500 Internal Server Error");
    assert_eq!(response.body, r#"{"error":"storage","field":"nvs"}"#);
    assert_eq!(rig.core.config(), before);
}

#[test]
fn malformed_bodies_and_fields_have_cpp_errors() {
    let mut rig = Rig::new();
    for body in [b"".as_slice(), b"{", &[b' '; MAX_BODY + 1]] {
        assert_eq!(
            post_action(&mut rig, body).body,
            r#"{"error":"invalid_json"}"#
        );
        assert_eq!(
            post_config(&mut rig, body).body,
            r#"{"error":"invalid_json"}"#
        );
    }
    for (body, field) in [
        (r#"[]"#, "action"),
        (r#"{"action":"unknown"}"#, "action"),
        (r#"{"action":"set_brew_actuators","pump_pct":20}"#, "ttl_ms"),
        (
            r#"{"action":"set_brew_actuators","pump_pct":20.1,"ttl_ms":100}"#,
            "pump_pct",
        ),
        (r#"{"action":"set_heating","on":true}"#, "duration_ms"),
        (
            r#"{"action":"set_heating","on":false,"duration_ms":100}"#,
            "duration_ms",
        ),
        (r#"{"action":"stop_brew","extra":true}"#, "extra"),
    ] {
        let response = post_action(&mut rig, body.as_bytes());
        assert_eq!(response.status, "400 Bad Request", "{body}");
        let json = json::parse(response.body.as_bytes()).unwrap();
        assert_eq!(
            json.get("field").and_then(Json::as_str),
            Some(field),
            "{body}"
        );
    }
}

#[test]
fn duplicates_keep_cpp_first_lookup_and_last_patch_semantics() {
    let mut rig = Rig::new();
    let response = post_config(
        &mut rig,
        br#"{"version":11,"brew":{"target_weight_g":40,"target_weight_g":42}} trailing"#,
    );
    assert_eq!(response.status, "200 OK");
    assert_eq!(rig.core.config().target_weight_g, 42.0);
    // Le premier `action` décide du type, même quand un doublon suit.
    let response = post_action(&mut rig, br#"{"action":"start_brew","action":"stop_brew"}"#);
    assert_eq!(response.status, "409 Conflict");
    assert_eq!(response.body, r#"{"ok":false,"reason":"unavailable"}"#);
}

#[test]
fn http_never_starts_brew_and_config_firmware_reject_active_cycles() {
    let mut rig = Rig::new();
    rig.ready();
    assert_eq!(
        post_action(&mut rig, br#"{"action":"start_brew"}"#).body,
        r#"{"ok":false,"reason":"unavailable"}"#
    );
    let clock = rig.clock();
    let result = rig.core.perform_action(
        clock,
        &ActionCommand::new(Action::StartBrew),
        ActionSource::Ui,
    );
    assert_eq!(result.reason, "ok");
    assert_eq!(
        post_config(&mut rig, br#"{"version":11}"#).status,
        "409 Conflict"
    );
    assert_eq!(
        begin_firmware_upload(&mut rig.core, clock, Some("target=screen"), 1024, false)
            .unwrap_err()
            .body,
        r#"{"error":"busy"}"#
    );
    assert_eq!(
        post_action(&mut rig, br#"{"action":"stop_brew"}"#).status,
        "200 OK"
    );

    let mut rig = Rig::new();
    rig.ready();
    assert_eq!(
        post_action(&mut rig, br#"{"action":"purge_press"}"#).status,
        "200 OK"
    );
    assert_eq!(
        post_config(&mut rig, br#"{"version":11}"#).status,
        "409 Conflict"
    );
    assert_eq!(
        post_action(&mut rig, br#"{"action":"purge_release"}"#).status,
        "200 OK"
    );
}

#[test]
fn bearer_authentication_requires_the_complete_exact_header() {
    let mut rig = Rig::new();
    assert!(authorize(&mut rig.core, Some("Bearer test-token"), "test-token").is_ok());
    for header in [
        None,
        Some("bearer test-token"),
        Some("Bearer test-toke"),
        Some("Bearer test-token "),
    ] {
        let response = authorize(&mut rig.core, header, "test-token").unwrap_err();
        assert_eq!(response.status, "401 Unauthorized");
        assert_eq!(response.body, r#"{"error":"unauthorized"}"#);
    }
}

#[test]
fn config_schema_and_values_are_validated_before_persistence() {
    let mut rig = Rig::new();
    let before = rig.core.config();
    for (body, error, field) in [
        (r#"{}"#, "invalid_value", "version"),
        (r#"{"version":9}"#, "unknown_version", "version"),
        (
            r#"{"version":11,"brew":{"target_weight_g":0}}"#,
            "invalid_value",
            "brew.target_weight_g",
        ),
        (
            r#"{"version":11,"heating":{"enabled":1}}"#,
            "invalid_value",
            "heating.enabled",
        ),
    ] {
        let response = post_config(&mut rig, body.as_bytes());
        assert_eq!(response.status, "400 Bad Request");
        let json = json::parse(response.body.as_bytes()).unwrap();
        assert_eq!(json.get("error").and_then(Json::as_str), Some(error));
        assert_eq!(json.get("field").and_then(Json::as_str), Some(field));
        assert_eq!(rig.core.config(), before);
    }
}

#[test]
fn serde_reader_handles_unicode_bom_duplicates_and_rejects_invalid_json() {
    let root = json::parse(b"\xef\xbb\xbf{\"x\":\"\\ud83d\\ude00\",\"x\":false}ignored").unwrap();
    assert_eq!(root.get("x").and_then(Json::as_str), Some("😀"));
    let Json::Object(pairs) = root else {
        panic!("object");
    };
    assert_eq!(pairs.len(), 2);
    for invalid in [
        b"{\"x\":01}".as_slice(),
        b"{\"x\":\"\x01\"}",
        b"{\"x\":\"\\ud800\"}",
    ] {
        assert!(json::parse(invalid).is_none());
    }
    let deeply_nested = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    assert!(json::parse(deeply_nested.as_bytes()).is_none());
}

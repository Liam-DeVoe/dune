//! While Prescience waits for Voice, the Atreides get a waiting "Use Prescience" form that says
//! why, and the Bene Gesserit get the form to decline Voice (behavior-changes.md, "Voice before
//! Prescience").

mod common;

use common::*;
use treachery_engine::prelude::*;
use treachery_view::schema::Part;

fn types(view: &treachery_view::schema::GameView) -> Vec<String> {
    view.actions.iter().map(|a| a.event_type.clone()).collect()
}

fn text(parts: &[Part]) -> String {
    serde_json::to_string(parts).unwrap()
}

#[test]
fn prescience_shows_as_waiting_until_voice_is_declined() {
    // worm-ride: the Bene Gesserit attack the Atreides; event 120 is the Bene Gesserit's Voice.
    let mut r = load_text(&truncated("worm-ride", 120));
    let (atreides, bg) = (viewer_of(&r, Faction::Atreides), viewer_of(&r, Faction::BeneGesserit));

    let view = r.view(atreides);
    assert!(!types(&view).contains(&"Prescience".to_string()));
    assert_eq!(view.waiting_actions.len(), 1);
    let waiting = &view.waiting_actions[0];
    assert_eq!((waiting.event_type.as_str(), waiting.title.as_str()), ("Prescience", "Use Prescience"));
    let reason = text(&waiting.reason);
    assert!(reason.contains("BeneGesserit") && reason.contains("use or decline Voice"), "{reason}");

    assert!(r.view(bg).waiting_actions.is_empty());
    assert!(r.view(OBSERVER).waiting_actions.is_empty());
    assert!(types(&r.view(bg)).contains(&"VoiceDeclined".to_string()));

    r.apply(r#"{"$type":"VoiceDeclined","Initiator":60,"Time":"2026-09-19T20:59:58.974212-04:00"}"#).unwrap();
    let view = r.view(atreides);
    assert!(view.waiting_actions.is_empty());
    assert!(types(&view).contains(&"Prescience".to_string()));
}

use super::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

fn fixture() -> String {
    let mut list: Vec<_> = (1..=99)
        .map(|i| {
            serde_json::json!({
                "tag_name": format!("v2026.10.14.{i}"),
                "name": format!("Build {i}"), "body": "Release notes\r\n".repeat(256),
                "published_at": "2026-10-14T12:00:00Z", "draft": i == 99,
                "assets": [{"name": ASSET.unwrap_or("unsupported"),
                    "browser_download_url": "https://example.test/game", "size": 12345}]
            })
        })
        .collect();
    list.push(serde_json::json!({
        "tag_name": "launcher@99.0.0", "name": "Launcher", "body": "Notes",
        "assets": [{"name": LAUNCHER_ASSET.unwrap_or("unsupported"),
            "browser_download_url": "https://example.test/launcher", "size": 6789}]
    }));
    serde_json::to_string(&list).unwrap()
}

fn installed_fixture() -> Vec<String> {
    vec!["v2026.10.14.50".into()]
}

/// The startup read: the game's list and the launcher's own are loaded separately and parsed
/// into the successful result each source would give on its own.
fn shared(
    game: impl FnOnce() -> Result<serde_json::Value, String>,
    launcher: impl FnOnce() -> Result<serde_json::Value, String>,
) -> (Vec<Release>, Updates) {
    let mut releases = None;
    let updates = releases_and_updates_with(
        game,
        launcher,
        installed_fixture,
        |list| parse_releases(list, LAUNCHER_ASSET, is_launcher_tag),
        |r| releases = Some(r.unwrap()),
    )
    .unwrap();
    (releases.unwrap(), updates)
}

#[test]
fn separate_lists_preserve_the_startup_results() {
    let text = fixture();
    let requests = AtomicUsize::new(0);
    let load = || {
        requests.fetch_add(1, Ordering::Relaxed);
        Ok(serde_json::from_str(&text).unwrap())
    };
    let (releases, updates) = shared(load, load);
    assert_eq!(requests.load(Ordering::Relaxed), 2);
    let list = serde_json::from_str::<serde_json::Value>(&text).unwrap();
    assert_eq!(
        serde_json::to_value(&releases).unwrap(),
        serde_json::to_value(parse_releases(&list, ASSET, is_game_tag)).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&updates).unwrap(),
        serde_json::to_value(&Updates {
            horizon: horizon_update(
                &parse_releases(&list, ASSET, is_game_tag),
                &installed_fixture(),
            ),
            launcher: launcher_update(&parse_releases(&list, LAUNCHER_ASSET, is_launcher_tag)),
        })
        .unwrap()
    );
}

#[test]
fn errors_reach_releases_and_updates_without_fallback() {
    for error in [
        "No connection: offline",
        "GitHub's rate limit was reached.",
        "invalid JSON",
    ] {
        let mut published = None;
        let result = releases_and_updates_with(
            || Err(error.into()),
            || panic!("the launcher's list is not read after a game failure"),
            || panic!("no installed scan after failure"),
            |_| panic!("no fallback after failure"),
            |r| published = Some(r),
        );
        assert_eq!(published.unwrap().unwrap_err(), error);
        assert_eq!(result.unwrap_err(), error);
    }
}

#[test]
fn a_failure_of_the_launchers_own_list_keeps_the_game_releases() {
    for error in ["No connection: offline", "GitHub's rate limit was reached."] {
        let mut published = None;
        let result = releases_and_updates_with(
            || Ok(serde_json::json!([{"tag_name": "v2026.10.14", "assets": []}])),
            || Err(error.to_string()),
            Vec::new,
            |_| panic!("the launcher's list is not parsed after failure"),
            |r| published = Some(r),
        );
        assert_eq!(published.unwrap().unwrap()[0].tag, "v2026.10.14");
        assert_eq!(result.unwrap_err(), error);
    }
}

#[test]
fn publish_games_before_older_launcher_fallback() {
    let events = RefCell::new(Vec::new());
    let result = releases_and_updates_with(
        || Ok(serde_json::json!([{"tag_name": "v2026.10.14", "assets": []}])),
        // A launcher tag without the platform's asset sends the lookup to the tags.
        || Ok(serde_json::json!([{"tag_name": "launcher@98.0.0", "assets": []}])),
        Vec::new,
        |list| {
            launcher_releases_in_with(list, |url| {
                assert_eq!(events.borrow().first(), Some(&"releases"));
                events.borrow_mut().push("fallback");
                if url.contains("matching-refs") {
                    Ok(serde_json::json!([{"ref": "refs/tags/launcher@99.0.0"}]))
                } else {
                    Ok(
                        serde_json::json!({"tag_name": "launcher@99.0.0", "assets": [{
                            "name": LAUNCHER_ASSET.unwrap_or("unsupported"),
                            "browser_download_url": "u", "size": 1
                        }]}),
                    )
                }
            })
        },
        |r| {
            assert_eq!(r.unwrap()[0].tag, "v2026.10.14");
            events.borrow_mut().push("releases");
        },
    )
    .unwrap();
    assert_eq!(*events.borrow(), ["releases", "fallback", "fallback"]);
    assert_eq!(result.launcher.is_some(), LAUNCHER_ASSET.is_some());
}

#[test]
fn failed_launcher_fallback_keeps_game_result_and_checks_stay_fresh() {
    let requests = AtomicUsize::new(0);
    for day in [14, 15] {
        let mut published = None;
        let updates = releases_and_updates_with(
            || {
                requests.fetch_add(1, Ordering::Relaxed);
                Ok(serde_json::json!([{"tag_name": format!("v2026.10.{day}")}]))
            },
            || Ok(serde_json::json!([{"tag_name": "launcher@98.0.0", "assets": []}])),
            Vec::new,
            |list| launcher_releases_in_with(list, |_| Err("offline".into())),
            |r| published = Some(r.unwrap()),
        )
        .unwrap();
        assert_eq!(published.unwrap()[0].tag, format!("v2026.10.{day}"));
        assert!(updates.launcher.is_none());
    }
    assert_eq!(requests.load(Ordering::Relaxed), 2);
}

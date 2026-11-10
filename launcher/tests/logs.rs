//! `horizon-launcher logs`: the end of the game's latest.log, and its newest crash report.
use std::path::Path;
use std::process::Command;

fn logs(home: &Path, args: &[&str]) -> (bool, String, String) {
    // Every CLI child takes the launcher's single-instance lock. Give it a private data
    // directory so parallel integration tests cannot make an otherwise valid logs call fail.
    let launcher_home = std::env::temp_dir().join(format!(
        "horizon-launcher-logs-child-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let out = Command::new(env!("CARGO_BIN_EXE_horizon-launcher"))
        .arg("logs")
        .args(args)
        .env("HORIZON_HOME", home)
        .env("HORIZON_LAUNCHER_HOME", &launcher_home)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&launcher_home);
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn logs_prints_the_log_tail_and_the_newest_crash_report() {
    let home = std::env::temp_dir().join(format!(
        "horizon-launcher-logs-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&home);

    let (ok, _, err) = logs(&home, &[]);
    assert!(!ok);
    assert!(err.contains("latest.log"), "{err}");

    std::fs::create_dir_all(home.join("logs")).unwrap();
    let text: String = (1..=150)
        .map(|i| format!("[10:00:00] [Render thread/INFO]: line {i}\n"))
        .collect();
    std::fs::write(home.join("logs/latest.log"), text).unwrap();
    let (ok, out, _) = logs(&home, &[]);
    assert!(ok, "stdout: {out}\nstderr: {}", logs(&home, &[]).2);
    assert_eq!(out.lines().count(), 100);
    assert!(out.starts_with("[10:00:00] [Render thread/INFO]: line 51\n"));
    let (_, out, _) = logs(&home, &["--lines", "1"]);
    assert_eq!(out, "[10:00:00] [Render thread/INFO]: line 150\n");
    let (_, out, _) = logs(&home, &["--json", "--lines", "1"]);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["text"], "[10:00:00] [Render thread/INFO]: line 150\n");
    assert!(json["path"].as_str().unwrap().ends_with("latest.log"));

    let (ok, _, err) = logs(&home, &["--crash"]);
    assert!(!ok);
    assert!(err.contains("No crash report"), "{err}");
    std::fs::create_dir_all(home.join("crash-reports")).unwrap();
    std::fs::write(
        home.join("crash-reports/crash-2026-10-13_08.00.00-client.txt"),
        "old\n",
    )
    .unwrap();
    std::fs::write(
        home.join("crash-reports/crash-2026-10-14_08.00.00-client.txt"),
        "new\n",
    )
    .unwrap();
    let (ok, out, _) = logs(&home, &["--crash"]);
    assert!(ok);
    assert_eq!(out, "new\n");
    let _ = std::fs::remove_dir_all(&home);
}

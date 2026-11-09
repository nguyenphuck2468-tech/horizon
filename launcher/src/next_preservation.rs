//! N01 — the executable preservation baseline for the next-generation shell.
//!
//! The rebrand charter keeps every feature of the current launcher. `next_navigation` names the
//! areas; this module turns that inventory into coverage: each workflow lists the production
//! entry point it uses and either a smoke test that calls that entry point or the reason a unit
//! test cannot, with the suite that covers it meanwhile.
//!
//! The smoke tests use no network, GPU or game process. The tests below run every one of them,
//! so a renamed or broken entry point fails here before a new shell is connected — and none of
//! them may be removed until its workflow has a replacement and a parity test.

use crate::next_navigation::FeatureArea;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How a preserved workflow is kept honest.
#[derive(Clone, Copy)]
pub enum Automation {
    /// A smoke test that calls the production entry point named beside it.
    Smoke(fn()),
    /// A path a unit test cannot exercise; the reason says what it needs and what covers it.
    Manual(&'static str),
}

/// One workflow that must keep working while the shell is replaced.
pub struct Workflow {
    pub area: FeatureArea,
    /// What the player does, in one line.
    pub name: &'static str,
    /// The production entry point the new shell must keep calling.
    pub entry_point: &'static str,
    pub automation: Automation,
}

/// Every workflow in the preservation contract.
pub const WORKFLOWS: &[Workflow] = &[
    Workflow {
        area: FeatureArea::AccountsAndIdentity,
        name: "An Ely.by sign-in is stored as Ely.by and exports no secret",
        entry_point: "account::finish_ely_for_cli, account::Secrets",
        automation: Automation::Smoke(ely_sign_in_stays_separate),
    },
    Workflow {
        area: FeatureArea::AccountsAndIdentity,
        name: "Microsoft device-code sign-in against the live services",
        entry_point: "account::device_code, account::poll, account::finish_sign_in",
        automation: Automation::Manual(
            "needs live Microsoft, Xbox Live and Minecraft services and a real account; account::tests parse their recorded fixtures",
        ),
    },
    Workflow {
        area: FeatureArea::VersionsAndBuilds,
        name: "A Nightly keeps its version, date and player-facing label",
        entry_point: "nightly::version_of, nightly::date_of, nightly::label",
        automation: Automation::Smoke(nightly_version_names_are_stable),
    },
    Workflow {
        area: FeatureArea::VersionsAndBuilds,
        name: "Launcher and game tags keep their release order",
        entry_point: "backend::is_game_tag, backend::launcher_version, backend::compare_versions",
        automation: Automation::Smoke(launcher_and_game_tags_keep_their_order),
    },
    Workflow {
        area: FeatureArea::VersionsAndBuilds,
        name: "Launcher self-update downloads and installs a signed release",
        entry_point: "backend::install, signed::launcher_digest",
        automation: Automation::Manual(
            "needs a published signed release, an install it can replace and the release workflow; signed::tests pin the signature checks",
        ),
    },
    Workflow {
        area: FeatureArea::InstallPrepareLaunch,
        name: "Prepared resources are reported from the real state",
        entry_point: "backend::resources_summary",
        automation: Automation::Smoke(prepared_state_is_reported),
    },
    Workflow {
        area: FeatureArea::InstallPrepareLaunch,
        name: "A real game download, prepare and launch",
        entry_point: "backend::install, backend::prepare_step, launch::plan",
        automation: Automation::Manual(
            "needs a Minecraft install, network and a running game window; backend and launch tests cover the pure planning pieces",
        ),
    },
    Workflow {
        area: FeatureArea::Instances,
        name: "Instance metadata written by older launchers still loads",
        entry_point: "instances::Meta, instances::play_time",
        automation: Automation::Smoke(instance_metadata_keeps_reading_older_files),
    },
    Workflow {
        area: FeatureArea::Instances,
        name: "Create, clone, import, export and delete an instance on disk",
        entry_point: "instances::create, instances::delete_with_selection, import, export",
        automation: Automation::Manual(
            "mutates the player's data directory; instances::tests and import/export tests cover the folder contracts",
        ),
    },
    Workflow {
        area: FeatureArea::ModsAndPacks,
        name: "Pack files keep their resource and shader kinds",
        entry_point: "packs::kind_of, packs::Kind",
        automation: Automation::Smoke(pack_files_keep_their_kinds),
    },
    Workflow {
        area: FeatureArea::ModsAndPacks,
        name: "Modrinth, CurseForge and Horizon catalogue downloads",
        entry_point: "browser, github_mods, modpacks",
        automation: Automation::Manual(
            "needs live provider APIs and rate limits; browser/tests and github_mods/tests pin the response parsing",
        ),
    },
    Workflow {
        area: FeatureArea::WorldsAndScreenshots,
        name: "A world backs up and screenshots list only pictures",
        entry_point: "worlds::backup, screenshots::list",
        automation: Automation::Smoke(world_backup_and_screenshot_listing_work),
    },
    Workflow {
        area: FeatureArea::Multiplayer,
        name: "The official server offer appears once and can be found",
        entry_point: "server_list::add_official, server_list::find",
        automation: Automation::Smoke(official_server_offer_is_idempotent),
    },
    Workflow {
        area: FeatureArea::Multiplayer,
        name: "Joining a live server and reading its status",
        entry_point: "server_list, telemetry::server_players",
        automation: Automation::Manual(
            "needs a running Minecraft server and a game client; server_list::tests cover the servers.dat format",
        ),
    },
    Workflow {
        area: FeatureArea::ServersAndNetwork,
        name: "Server names and Tailscale sharing rules stay stable",
        entry_point: "servers::slug, tailscale::reach, tailscale::limit_shared",
        automation: Automation::Smoke(server_names_and_tailscale_rules_hold),
    },
    Workflow {
        area: FeatureArea::ServersAndNetwork,
        name: "Starting a hosted server and changing the live tailnet",
        entry_point: "servers::set_up, servers::run, tailscale::status",
        automation: Automation::Manual(
            "needs the Tailscale CLI logged in, a server process and admin rights; tailscale::tests pin the status and policy parsing",
        ),
    },
    Workflow {
        area: FeatureArea::JavaAndLaunchSettings,
        name: "Memory limits and environment variables round-trip",
        entry_point: "launch::parse_memory, launch::memory_at, launch::env_line",
        automation: Automation::Smoke(launch_memory_and_env_round_trip),
    },
    Workflow {
        area: FeatureArea::JavaAndLaunchSettings,
        name: "Picking and installing a real Java runtime",
        entry_point: "java::pick, java::install",
        automation: Automation::Manual(
            "needs Mojang runtime downloads and an installed Java home; java::tests build fake homes for the selection rules",
        ),
    },
    Workflow {
        area: FeatureArea::ConsoleAndDiagnostics,
        name: "Native mod build state is explicit",
        entry_point: "native_status::state, native_status::State",
        automation: Automation::Smoke(native_mod_build_state_is_explicit),
    },
    Workflow {
        area: FeatureArea::ConsoleAndDiagnostics,
        name: "Capturing logs and crash reports from a running game",
        entry_point: "game_log, console, native_log_page",
        automation: Automation::Manual(
            "needs a running game process and a GPU; game_log::tests pin the log parsing and formatting",
        ),
    },
    Workflow {
        area: FeatureArea::SkinsAndCosmetics,
        name: "An offline skin reaches the game only while it is kept",
        entry_point: "skins::keep, skins::env, skins::forget",
        automation: Automation::Smoke(offline_skins_reach_the_game_only_when_kept),
    },
    Workflow {
        area: FeatureArea::SkinsAndCosmetics,
        name: "Claiming wardrobe cosmetics from the live service",
        entry_point: "wardrobe::Service",
        automation: Automation::Manual(
            "needs a signed-in account and the live cosmetics API; wardrobe::tests pin the catalogue and claim states",
        ),
    },
    Workflow {
        area: FeatureArea::ConfigurationAndPlatform,
        name: "Client IDs resolve from config, environment, then default",
        entry_point: "config::parse_config_text, config::resolve_client_id",
        automation: Automation::Smoke(configuration_precedence_is_unchanged),
    },
    Workflow {
        area: FeatureArea::ConfigurationAndPlatform,
        name: "Keychain storage, installers and OS integration",
        entry_point: "account::SystemStore, installer, desktop",
        automation: Automation::Manual(
            "needs the OS keychain, a real installer and a desktop session; account::tests use an in-memory store for the same interface",
        ),
    },
];

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "horizon-preservation-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn ely_sign_in_stays_separate() {
    let tokens = crate::ely::Tokens {
        access_token: "access-token-for-test".into(),
        refresh_token: Some("refresh-token-for-test".into()),
        expires_at: 1_900_000_000,
        profile: crate::ely::Profile {
            uuid: "0123456789abcdef0123456789abcdef".into(),
            name: "ElyTester".into(),
        },
    };
    let (account, secrets) = crate::account::finish_ely_for_cli(tokens);
    assert_eq!(account.uuid, "0123456789abcdef0123456789abcdef");
    assert_eq!(account.name, "ElyTester");

    // The keychain payload carries the provider marker `account::is_ely` reads later, and Ely.by
    // never confirms Minecraft ownership: it may play through the separate `plays` entitlement.
    let stored = serde_json::to_value(&secrets).unwrap();
    assert!(
        stored["minecraft_token"]
            .as_str()
            .is_some_and(|token| token.starts_with("elyby:")),
        "{stored}"
    );
    assert!(
        stored["refresh_token"]
            .as_str()
            .is_some_and(|token| token.starts_with("elyby-refresh:")),
        "{stored}"
    );
    assert_eq!(stored["owns"], serde_json::json!(false));
    assert_eq!(stored["plays"], serde_json::json!(true));

    let summary = serde_json::to_string(&account).unwrap();
    assert!(!summary.contains("access-token-for-test"));
    assert!(!summary.contains("refresh-token-for-test"));
    assert!(!summary.to_ascii_lowercase().contains("token"));
    assert_eq!(
        serde_json::from_str::<crate::account::Account>(&summary).unwrap(),
        account
    );
    assert_eq!(format!("{secrets:?}"), "Secrets(<hidden>)");
}

fn nightly_version_names_are_stable() {
    let release = crate::backend::Release {
        tag: "nightly".into(),
        name: "Nightly 2026-10-28 · 0c02067".into(),
        body: String::new(),
        date: "2026-10-28T00:00:00Z".into(),
        time: String::new(),
        asset: None,
        prerelease: true,
        nightly: Some("nightly".into()),
    };
    let version = crate::nightly::version_of(&release);
    assert_eq!(version, "nightly-2026-10-28-0c02067");
    assert_eq!(crate::nightly::date_of(&version), Some("2026-10-28"));
    assert_eq!(
        crate::nightly::label(&version).as_deref(),
        Some("Nightly · 0c02067")
    );
    assert_eq!(crate::nightly::label("launcher@0.2.1"), None);
    assert_eq!(crate::nightly::date_of("launcher@0.2.1"), None);
}

fn launcher_and_game_tags_keep_their_order() {
    assert!(crate::backend::is_game_tag("v2026.10.13"));
    assert!(!crate::backend::is_game_tag("launcher@0.2.54"));
    assert_eq!(
        crate::backend::launcher_version("launcher@0.2.54"),
        Some("0.2.54")
    );
    assert_eq!(crate::backend::launcher_version("v0.2.54"), None);
    assert_eq!(
        crate::backend::compare_versions("nightly-2026-10-29-abc", "launcher@0.2.54"),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        crate::backend::compare_versions("launcher@0.2.1", "launcher@0.2.0"),
        std::cmp::Ordering::Greater
    );
}

fn prepared_state_is_reported() {
    let fresh = crate::backend::resources_summary(false, false, "1.21.4");
    assert!(fresh.contains("Not prepared"), "{fresh}");
    assert!(fresh.contains("1.21.4"), "{fresh}");
    let silent = crate::backend::resources_summary(true, false, "1.21.4");
    assert!(silent.contains("without its sounds"), "{silent}");
    let ready = crate::backend::resources_summary(true, true, "1.21.4");
    assert!(ready.starts_with("Prepared for Minecraft"), "{ready}");
    assert!(!ready.contains("without its sounds"), "{ready}");
}

fn instance_metadata_keeps_reading_older_files() {
    let meta: crate::instances::Meta =
        serde_json::from_str(r#"{"name":"Survival","played_secs":90}"#).unwrap();
    assert_eq!(meta.name.as_deref(), Some("Survival"));
    assert_eq!(meta.played_secs, 90);
    assert_eq!(meta.java, None);

    let old: crate::instances::Meta = serde_json::from_str("{}").unwrap();
    assert_eq!(old, crate::instances::Meta::default());
    let written = serde_json::to_value(&old).unwrap();
    assert!(
        written.get("played_secs").is_none(),
        "defaults stay out of meta.json: {written}"
    );

    assert_eq!(
        crate::instances::play_time(Duration::from_secs(12 * 60)),
        "12 min"
    );
    assert_eq!(
        crate::instances::play_time(Duration::from_secs(3 * 3600 + 5 * 60)),
        "3 h 05 min"
    );
    assert_eq!(
        crate::instances::play_time(Duration::from_secs(140 * 3600)),
        "140 h"
    );
}

fn pack_files_keep_their_kinds() {
    let root = scratch("packs");
    let resource = root.join("pack.zip");
    write_zip(&resource, &[("pack.mcmeta", b"{}")]);
    assert_eq!(
        crate::packs::kind_of(&resource),
        Ok(crate::packs::Kind::Resource)
    );

    let shader = root.join("shader.zip");
    write_zip(&shader, &[("shaders/water.fsh", b"void main() {}")]);
    assert_eq!(
        crate::packs::kind_of(&shader),
        Ok(crate::packs::Kind::Shader)
    );

    let horizon = root.join("water.hshader");
    std::fs::write(&horizon, b"hshader").unwrap();
    assert_eq!(
        crate::packs::kind_of(&horizon),
        Ok(crate::packs::Kind::Shader)
    );

    let jar = root.join("mod.jar");
    std::fs::write(&jar, b"PK").unwrap();
    let error = crate::packs::kind_of(&jar).unwrap_err();
    assert!(
        error.contains(".zip") && error.contains(".hshader"),
        "a mod jar is not a pack: {error}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    for (name, bytes) in files {
        zip.start_file(*name, options).unwrap();
        std::io::Write::write_all(&mut zip, bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn world_backup_and_screenshot_listing_work() {
    let home = scratch("worlds");
    let folder = home.join("saves").join("Survival");
    std::fs::create_dir_all(folder.join("region")).unwrap();
    std::fs::write(folder.join("level.dat"), b"level").unwrap();
    std::fs::write(folder.join("region").join("r.0.0.mca"), b"region").unwrap();

    let world = crate::worlds::World {
        folder: "Survival".into(),
        path: folder,
        name: "Survival".into(),
        mode: "Survival",
        last_played: SystemTime::UNIX_EPOCH,
        version: None,
        size: 0,
        icon: None,
    };
    let backup = crate::worlds::backup(&home, &world).unwrap();
    assert!(backup.starts_with(home.join("backups")), "{backup:?}");
    assert!(backup.is_file());

    let shots = crate::screenshots::folder(&home);
    std::fs::create_dir_all(&shots).unwrap();
    std::fs::write(shots.join("2026-10-28_10.00.00.png"), b"png").unwrap();
    std::fs::write(shots.join("notes.txt"), b"text").unwrap();
    let names: Vec<_> = crate::screenshots::list(&home)
        .into_iter()
        .map(|shot| shot.name)
        .collect();
    assert_eq!(names, vec!["2026-10-28_10.00.00.png".to_string()]);
    assert_eq!(
        crate::screenshots::find(&home, "2026-10-28_10.00.00")
            .unwrap()
            .size,
        3
    );
    std::fs::remove_dir_all(home).unwrap();
}

fn official_server_offer_is_idempotent() {
    let home = scratch("servers");
    let address = crate::server_list::OFFICIAL_ADDRESS;
    crate::server_list::add_official(&home, address, &[]).unwrap();

    let list = crate::server_list::List::load(&home);
    let (index, server) =
        crate::server_list::find(&list, crate::server_list::OFFICIAL_NAME).unwrap();
    assert_eq!(index, 0);
    assert_eq!(server.ip, address);

    crate::server_list::add_official(&home, address, &[]).unwrap();
    let again = crate::server_list::List::load(&home);
    assert_eq!(crate::server_list::find(&again, address).unwrap().0, 0);
    std::fs::remove_dir_all(home).unwrap();
}

fn server_names_and_tailscale_rules_hold() {
    assert_eq!(crate::servers::slug("My Server!"), "my-server");
    assert_eq!(crate::servers::slug("  "), "server");

    let policy = serde_json::json!({
        "acls": [{ "action": "accept", "src": ["*"], "dst": ["*:*"] }]
    });
    assert_eq!(
        crate::tailscale::reach(&policy, "100.64.0.1", 25565),
        crate::tailscale::Reach::Everything
    );
    let limited = crate::tailscale::limit_shared(&policy, "100.64.0.1", 25565).unwrap();
    assert_eq!(
        crate::tailscale::reach(&limited, "100.64.0.1", 25565),
        crate::tailscale::Reach::ServerOnly
    );
    assert!(crate::tailscale::snippet("100.64.0.1", 25565).contains("\"100.64.0.1:25565\""));

    let custom = serde_json::json!({
        "acls": [{ "action": "accept", "src": ["group:friends"], "dst": ["100.64.0.1:25565"] }]
    });
    assert_eq!(
        crate::tailscale::reach(&custom, "100.64.0.1", 25565),
        crate::tailscale::Reach::Custom
    );
}

fn launch_memory_and_env_round_trip() {
    assert_eq!(crate::launch::parse_memory("4G"), Ok(Some(4096)));
    assert_eq!(crate::launch::parse_memory("6144 mb"), Ok(Some(6144)));
    assert_eq!(crate::launch::parse_memory("off"), Ok(None));
    assert!(crate::launch::parse_memory("plenty").is_err());
    assert_eq!(crate::launch::memory_at(0.5, 8192), Some(4096));
    assert_eq!(crate::launch::memory_at(1.0, 8192), None);
    assert!((crate::launch::memory_fraction(Some(2048), 8192) - 0.25).abs() < f32::EPSILON);

    let env: BTreeMap<String, String> =
        crate::launch::parse_env("MANGOHUD=1 NAME=\"two words\"").unwrap();
    assert_eq!(env["MANGOHUD"], "1");
    assert_eq!(env["NAME"], "two words");
    assert_eq!(
        crate::launch::parse_env(&crate::launch::env_line(&env)).unwrap(),
        env
    );
}

fn native_mod_build_state_is_explicit() {
    use crate::native_status::{state, Mod, State};
    let mods: Vec<Mod> = vec![("water".into(), "1.0".into(), Some("hash".into()))];
    assert_eq!(state(&[], None), State::NoMods);
    assert_eq!(state(&mods, None), State::NotBuilt);
    assert_eq!(state(&mods, Some(&mods)), State::UpToDate);
    let other: Vec<Mod> = vec![("water".into(), "2.0".into(), None)];
    assert_eq!(state(&mods, Some(&other)), State::Outdated);
}

fn offline_skins_reach_the_game_only_when_kept() {
    let root = scratch("skins");
    let dir = root.join("skins");
    let picture = root.join("alex.png");
    let skin = image::RgbaImage::from_fn(64, 64, |x, y| image::Rgba([x as u8, y as u8, 0, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    skin.write_to(&mut png, image::ImageFormat::Png).unwrap();
    std::fs::write(&picture, png.into_inner()).unwrap();

    let look = crate::skins::Look {
        model: crate::skins::Model::Slim,
        skin: true,
        cape: false,
    };
    assert!(crate::skins::env(&dir, "Alex", &look).is_empty());
    crate::skins::keep(&dir, "Alex", &picture, false).unwrap();
    let env = crate::skins::env(&dir, "Alex", &look);
    assert_eq!(env.len(), 2);
    assert_eq!(env[0].0, crate::skins::SKIN_ENV);
    assert_eq!(env[1].0, crate::skins::MODEL_ENV);
    assert_eq!(env[1].1, "slim");
    assert!(Path::new(&env[0].1).is_file());

    crate::skins::forget(&dir, "Alex");
    assert!(crate::skins::env(&dir, "Alex", &look).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

fn configuration_precedence_is_unchanged() {
    let text = "HORIZON_MSA_CLIENT_ID = \"msa\"\nHORIZON_ELY_CLIENT_ID = \"ely\"\n";
    let config = crate::config::parse_config_text(text);
    assert_eq!(config.horizon_msa_client_id.as_deref(), Some("msa"));
    assert_eq!(config.horizon_ely_client_id.as_deref(), Some("ely"));
    assert_eq!(
        crate::config::parse_config_text("not toml = [").horizon_msa_client_id,
        None
    );

    assert_eq!(
        crate::config::resolve_client_id(Some("config"), Some("env"), "default").as_deref(),
        Some("config")
    );
    assert_eq!(
        crate::config::resolve_client_id(Some("  "), Some("env"), "default").as_deref(),
        Some("env")
    );
    assert_eq!(
        crate::config::resolve_client_id(None, None, "default").as_deref(),
        Some("default")
    );
    assert_eq!(crate::config::resolve_client_id(None, Some(" "), " "), None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_preserved_area_has_a_workflow() {
        let covered: HashSet<FeatureArea> =
            WORKFLOWS.iter().map(|workflow| workflow.area).collect();
        for area in crate::next_navigation::PRESERVED_FEATURE_AREAS {
            assert!(covered.contains(area), "no workflow for {area:?}");
        }
        for area in &covered {
            assert!(
                crate::next_navigation::PRESERVED_FEATURE_AREAS.contains(area),
                "{area:?} is not in the preserved inventory"
            );
        }
    }

    #[test]
    fn automated_workflows_run_their_real_entry_points() {
        for workflow in WORKFLOWS {
            if let Automation::Smoke(run) = workflow.automation {
                let ran = std::panic::catch_unwind(run);
                assert!(ran.is_ok(), "preservation smoke failed: {}", workflow.name);
            }
        }
    }

    #[test]
    fn manual_workflows_say_what_a_test_needs() {
        for workflow in WORKFLOWS {
            if let Automation::Manual(reason) = workflow.automation {
                assert!(
                    reason.split_whitespace().count() >= 8,
                    "{}: name what a test cannot provide and what covers it",
                    workflow.name
                );
            }
        }
    }

    #[test]
    fn every_workflow_names_an_entry_point() {
        for workflow in WORKFLOWS {
            assert!(
                !workflow.entry_point.trim().is_empty(),
                "{}: no entry point",
                workflow.name
            );
        }
    }
}

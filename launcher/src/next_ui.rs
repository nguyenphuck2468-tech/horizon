//! First native GPUI shell for the replacement launcher frontend.
//!
//! This is deliberately a new screen tree, not a reskin of the legacy pages. It starts as a
//! navigation/layout preview while feature workflows are migrated through next_facade. Set
//! HORIZON_LAUNCHER_NEXT_UI=1 in a development build to open it. The final release cutover
//! removes this preview gate and the old renderer after feature-parity checks pass.

use super::*;
use gpui::{div, prelude::*, px, ClickEvent, SharedString};

fn preview_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Keep the new shell usable on compact laptop and split-screen windows.
/// Values are deliberately conservative to leave room for the actual page content.
fn layout_metrics(window_width: f32) -> (f32, f32, f32) {
    if window_width < 720. {
        (148., 14., 22.)
    } else if window_width < 1080. {
        (184., 22., 26.)
    } else {
        (232., 34., 30.)
    }
}

/// Whether the replacement shell is requested for local development and screenshot tests.
/// Require an explicit value so unrelated inherited environment variables cannot switch UIs.
pub(super) fn wanted() -> bool {
    preview_enabled(std::env::var("HORIZON_LAUNCHER_NEXT_UI").ok().as_deref())
}

fn label(route: horizon_launcher::next_navigation::Route) -> &'static str {
    use horizon_launcher::next_navigation::Route::*;
    match route {
        Home => "Home",
        InstanceLibrary => "Instances",
        InstanceDetails => "Instance details",
        VersionManager => "Versions",
        AccountCenter => "Accounts",
        AccountDetails => "Account details",
        ContentBrowser => "Content",
        InstalledContent => "Installed content",
        ModpackManager => "Modpacks",
        ModWorkshop => "Mod Workshop",
        ServerDashboard => "Servers",
        ServerConsole => "Server console",
        MultiplayerBrowser => "Multiplayer",
        WorldLibrary => "Worlds",
        ScreenshotLibrary => "Screenshots",
        Wardrobe => "Cosmetics",
        DiagnosticsOverview => "Diagnostics",
        DownloadQueue => "Downloads",
        CrashReports => "Crash reports",
        GlobalSettings => "Settings",
        InstanceSettings => "Instance settings",
    }
}

fn workspace_is_active(
    workspace: horizon_launcher::next_navigation::Workspace,
    active: horizon_launcher::next_navigation::Route,
) -> bool {
    horizon_launcher::next_navigation::ROUTES
        .iter()
        .any(|entry| entry.workspace == workspace && entry.route == active)
}

fn content_kind(
    route: horizon_launcher::next_navigation::Route,
) -> Option<horizon_launcher::browser::Kind> {
    use horizon_launcher::next_navigation::Route;

    match route {
        Route::ContentBrowser | Route::InstalledContent => {
            Some(horizon_launcher::browser::Kind::Mod)
        }
        Route::ModpackManager => Some(horizon_launcher::browser::Kind::Modpack),
        // Native .hnmod bundles use next_facade::native_mods, never the .hmod browser.
        _ => None,
    }
}

fn select_route(this: &mut Launcher, route: horizon_launcher::next_navigation::Route) {
    // Keep the established content services authoritative while the new visual panels are built.
    if let Some(kind) = content_kind(route) {
        this.mods_set_kind(kind);
    }
    this.next_route = route;
}

fn description(route: horizon_launcher::next_navigation::Route) -> &'static str {
    use horizon_launcher::next_navigation::Route::*;
    match route {
        Home => "Your game, instances and next action in one clear place.",
        InstanceLibrary | InstanceDetails | VersionManager | InstanceSettings =>
            "Manage game profiles, versions, Java and per-instance configuration.",
        AccountCenter | AccountDetails =>
            "Manage Microsoft, Ely.by and offline identities without exposing credentials to the UI.",
        ContentBrowser | InstalledContent | ModpackManager =>
            "Discover and manage validated Horizon content and mod packages.",
        ModWorkshop =>
            "Inspect native mods, resolve compatibility and run builds with live progress and logs.",
        ServerDashboard | ServerConsole | MultiplayerBrowser =>
            "Manage servers, multiplayer sessions and network diagnostics.",
        WorldLibrary | ScreenshotLibrary => "Browse worlds and captured game moments.",
        Wardrobe => "Manage skins and supported cosmetics.",
        DiagnosticsOverview | DownloadQueue | CrashReports =>
            "Inspect downloads, logs, failures and recovery information.",
        GlobalSettings => "Configure appearance, language, paths, Java and performance.",
    }
}

/// Render the replacement shell. Workflow panels are migrated incrementally through next_facade;
/// until then, this shell is an honest navigation/layout preview, not a fake working launcher.
pub(super) fn render(this: &mut Launcher, cx: &mut Context<Launcher>) -> impl IntoElement {
    use horizon_launcher::next_navigation::{Route, ROUTES};

    let t = this.theme;
    let (sidebar_width, page_padding, page_title_size) = layout_metrics(this.window_width);
    let active = this.next_route;
    let instance_name = this
        .selected_instance()
        .map(|instance| instance.name.clone())
        .unwrap_or_else(|| "No instance selected".into());
    let selected_id = this.selected_instance().map(|instance| instance.id.clone());
    let instances = this.instances.list.clone();
    let accounts = this.accounts.accounts.clone();
    let selected_account = this.accounts.selected.clone();
    let offline_names = this.accounts.offline.clone();
    let offline_selected = this.accounts.offline_selected.clone();
    let auth_status = this.auth_status.clone();
    let selected_version = this.selected().cloned();
    let game_running = this.client.is_some() || this.starting_client.is_some();
    let version_releases = match &this.releases {
        Load::Ready(releases) => releases.clone(),
        _ => Vec::new(),
    };
    let versions_loading = matches!(&this.releases, Load::Loading);
    let versions_error = match &this.releases {
        Load::Failed(error) => Some(error.clone()),
        _ => None,
    };
    let installed_versions = this.installed.clone();
    let installing_version = this.installing.clone();
    let removing_versions = this.removing.clone();
    let version_error = this.install_error.clone();
    let confirm_remove = this.confirm_remove.clone();
    let content_route = matches!(
        active,
        Route::ContentBrowser | Route::InstalledContent | Route::ModpackManager
    );
    let content_page = if content_route {
        Some(this.mods_page(cx))
    } else {
        None
    };
    // The native workshop has its own route and reuses only the trusted .hnmod build/import
    // services; it is not routed through the .hmod browser or .hpack profile manager.
    let native_workshop_page = if active == Route::ModWorkshop {
        let mut panel = div().flex().flex_col().gap(px(14.));
        if let Some(build) = this.native_build(cx) {
            panel = panel.child(build);
        }
        Some(panel.child(this.github_panel(cx)))
    } else {
        None
    };
    // Instance details are rendered in the replacement design and read only from the
    // selected instance; all launch and filesystem actions still delegate to existing services.
    let instance_details_page = if active == Route::InstanceDetails {
        Some(if let Some(instance) = this.selected_instance().cloned() {
            let version = this
                .version_of(&instance)
                .cloned()
                .unwrap_or_else(|| tr("No build selected").to_string());
            let home = instance.home.clone();
            let played = horizon_launcher::next_facade::instances::play_time(
                instance.played.as_secs(),
            );
            div()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .p(px(22.))
                        .rounded(px(14.))
                        .bg(t.surface)
                        .child(div().text_size(px(11.)).text_color(t.muted).child(tr("INSTANCE PROFILE")))
                        .child(div().font_family(theme::DISPLAY_FONT).text_size(px(25.)).text_color(t.ink).child(instance.name.clone()))
                        .child(div().text_size(px(12.)).text_color(t.muted).child(instance.id.clone())),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .children([
                            ("VERSION", version),
                            ("PLAY TIME", played),
                        ].into_iter().map(|(label, value)| {
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .gap(px(6.))
                                .p(px(16.))
                                .rounded(px(11.))
                                .bg(t.plate_soft)
                                .child(div().text_size(px(10.)).text_color(t.muted).child(tr(label)))
                                .child(div().text_size(px(15.)).text_color(t.ink).child(value))
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .p(px(16.))
                        .rounded(px(11.))
                        .bg(t.plate_soft)
                        .child(div().text_size(px(10.)).text_color(t.muted).child(tr("GAME DIRECTORY")))
                        .child(div().text_size(px(12.)).text_color(t.ink_soft).child(home.display().to_string())),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(
                            div()
                                .id("next-instance-details-play")
                                .px(px(16.))
                                .py(px(10.))
                                .rounded(px(8.))
                                .bg(t.accent)
                                .text_color(t.page)
                                .cursor_pointer()
                                .child(tr("Play"))
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.play();
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .id("next-instance-details-folder")
                                .px(px(16.))
                                .py(px(10.))
                                .rounded(px(8.))
                                .bg(t.plate)
                                .text_color(t.ink)
                                .cursor_pointer()
                                .child(tr("Open game folder"))
                                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                    crate::glass_toast::open(cx, &home);
                                })),
                        ),
                )
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .p(px(22.))
                .rounded(px(12.))
                .bg(t.plate_soft)
                .child(div().text_size(px(15.)).text_color(t.ink).child(tr("No instance selected")))
                .child(
                    div()
                        .id("next-instance-details-back")
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .bg(t.accent)
                        .text_color(t.page)
                        .cursor_pointer()
                        .child(tr("Browse instances"))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            select_route(this, Route::InstanceLibrary);
                            cx.notify();
                        })),
                )
        })
    } else {
        None
    };

    // Reuse the established device-code sign-in state machine and its safe actions. The new
    // account screen owns the presentation, while the existing provider flows remain authoritative.
    let account_sign_in_panel = if active == Route::AccountCenter {
        let (title, detail, actions) = this.sign_in_parts(cx);
        let mut panel = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(18.))
            .rounded(px(12.))
            .bg(t.plate_soft)
            .child(div().text_size(px(15.)).text_color(t.ink).child(title))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(t.ink_soft)
                    .child(detail),
            );
        if let Some(actions) = actions {
            panel = panel.child(actions);
        }
        Some(panel)
    } else {
        None
    };

    let mut navigation = div()
        .flex()
        .flex_col()
        .w(px(sidebar_width))
        .h_full()
        .p(px(18.))
        .gap(px(8.))
        .bg(t.plate_soft)
        .child(
            div()
                .px(px(12.))
                .py(px(18.))
                .font_family(theme::DISPLAY_FONT)
                .text_size(px(24.))
                .text_color(t.ink)
                .child("HORIZON"),
        )
        .child(
            div()
                .px(px(12.))
                .pb(px(12.))
                .text_size(px(11.))
                .text_color(t.muted)
                .child(tr("NEXT-GENERATION LAUNCHER")),
        );

    // ROUTES is ordered by workspace. Render one primary destination per workspace,
    // then nest its secondary destinations below it instead of flattening 21 routes.
    for entry in ROUTES.iter().filter(|entry| {
        !ROUTES
            .iter()
            .take_while(|candidate| candidate.route != entry.route)
            .any(|candidate| candidate.workspace == entry.workspace)
    }) {
        let workspace = entry.workspace;
        let primary = entry.route;
        let selected = workspace_is_active(workspace, active);
        navigation = navigation.child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    div()
                        .id(SharedString::from(format!("next-workspace-{}", entry.id)))
                        .w_full()
                        .px(px(12.))
                        .py(px(9.))
                        .rounded(px(8.))
                        .text_size(px(13.))
                        .text_color(if selected { t.accent } else { t.ink })
                        .when(selected, |element| element.bg(t.surface))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            select_route(this, primary);
                            cx.notify();
                        }))
                        .child(tr(label(primary))),
                )
                .children(
                    ROUTES
                        .iter()
                        .filter(|child| child.workspace == workspace && child.route != primary)
                        .map(|child| {
                            let route = child.route;
                            let selected = route == active;
                            div()
                                .id(SharedString::from(format!("next-route-{}", child.id)))
                                .w_full()
                                .pl(px(24.))
                                .pr(px(12.))
                                .py(px(7.))
                                .rounded(px(7.))
                                .text_size(px(12.))
                                .text_color(if selected { t.accent } else { t.ink_soft })
                                .when(selected, |element| element.bg(t.surface))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    select_route(this, route);
                                    cx.notify();
                                }))
                                .child(tr(label(route)))
                        }),
                ),
        );
    }

    let page = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .h_full()
        .p(px(page_padding))
        .gap(px(22.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .font_family(theme::DISPLAY_FONT)
                                .text_size(px(page_title_size))
                                .text_color(t.ink)
                                .child(tr(label(active))),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(t.muted)
                                .child(tr(description(active))),
                        ),
                )
                .child(
                    div()
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .bg(t.plate)
                        .text_color(t.ink_soft)
                        .text_size(px(12.))
                        .child(tr("Preview · feature migration in progress")),
                ),
        )
        .when(active == Route::Home, |page| {
            page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .p(px(24.))
                    .rounded(px(14.))
                    .bg(t.surface)
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.muted)
                            .child(tr("SELECTED INSTANCE")),
                    )
                    .child(
                        div()
                            .font_family(theme::DISPLAY_FONT)
                            .text_size(px(25.))
                            .text_color(t.ink)
                            .child(instance_name),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(t.ink_soft)
                            .child(format!(
                                "{}: {} · {}: {}",
                                tr("Version"),
                                tr(selected_version.as_deref().unwrap_or("No build selected")),
                                tr("Installed builds"),
                                this.installed.len()
                            )),
                    )
                    .child(
                        div()
                            .id("next-home-play")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(180.))
                            .px(px(18.))
                            .py(px(12.))
                            .rounded(px(9.))
                            .bg(t.accent)
                            .text_color(t.page)
                            .cursor_pointer()
                            .child(tr(if game_running { "Game starting / running" } else { "Play" }))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.play();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(4.))
                                    .p(px(13.))
                                    .rounded(px(10.))
                                    .bg(t.plate_soft)
                                    .child(div().text_size(px(22.)).text_color(t.ink).child(instances.len().to_string()))
                                    .child(div().text_size(px(11.)).text_color(t.muted).child(tr("Instances"))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(4.))
                                    .p(px(13.))
                                    .rounded(px(10.))
                                    .bg(t.plate_soft)
                                    .child(div().text_size(px(22.)).text_color(t.ink).child(installed_versions.len().to_string()))
                                    .child(div().text_size(px(11.)).text_color(t.muted).child(tr("Installed builds"))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(4.))
                                    .p(px(13.))
                                    .rounded(px(10.))
                                    .bg(t.plate_soft)
                                    .child(div().text_size(px(22.)).text_color(t.ink).child((accounts.len() + offline_names.len()).to_string()))
                                    .child(div().text_size(px(11.)).text_color(t.muted).child(tr("Accounts"))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .children([
                                (Route::InstanceLibrary, "Instances"),
                                (Route::VersionManager, "Versions"),
                                (Route::ContentBrowser, "Content"),
                                (Route::AccountCenter, "Accounts"),
                            ].into_iter().map(|(route, title)| {
                                div()
                                    .id(SharedString::from(format!("next-home-quick-{title}")))
                                    .flex()
                                    .flex_1()
                                    .justify_center()
                                    .px(px(12.))
                                    .py(px(10.))
                                    .rounded(px(8.))
                                    .bg(t.plate)
                                    .text_size(px(12.))
                                    .text_color(t.ink)
                                    .cursor_pointer()
                                    .child(tr(title))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        select_route(this, route);
                                        cx.notify();
                                    }))
                            })),
                    ),
            )
        })
        .when(active == Route::InstanceLibrary, |page| {
            page.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p(px(14.))
                    .rounded(px(10.))
                    .bg(t.plate_soft)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(div().text_size(px(14.)).text_color(t.ink).child(tr("Instances")))
                            .child(div().text_size(px(11.)).text_color(t.muted).child(tr("Manage game profiles, versions, Java and per-instance configuration."))),
                    )
                    .child(
                        div()
                            .id("next-instance-create")
                            .px(px(14.))
                            .py(px(9.))
                            .rounded(px(8.))
                            .bg(t.accent)
                            .text_color(t.page)
                            .cursor_pointer()
                            .child(tr("New instance"))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.open_instance_dialog(None);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(instances.iter().map(|instance| {
                        let id = instance.id.clone();
                        let editing = instance.clone();
                        let selected = selected_id.as_deref() == Some(id.as_str());
                        div()
                            .id(SharedString::from(format!("next-instance-{id}")))
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .p(px(16.))
                            .rounded(px(10.))
                            .bg(if selected { t.surface } else { t.plate_soft })
                            .text_color(t.ink)
                            .cursor_pointer()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(instance.name.clone())
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(t.muted)
                                            .child(format!("{} · {} {}", instance.id, instance.played.as_secs(), tr("seconds played"))),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(tr(if selected { "Selected" } else { "Select" }))
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("next-instance-edit-{id}")))
                                            .px(px(10.))
                                            .py(px(6.))
                                            .rounded(px(7.))
                                            .bg(t.plate)
                                            .text_size(px(11.))
                                            .child(tr("Edit instance"))
                                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                cx.stop_propagation();
                                                this.open_instance_dialog(Some(editing.clone()));
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if let Some(chosen) = this.instances.list.iter().find(|item| item.id == id).cloned() {
                                    this.select_instance(&chosen);
                                    this.next_route = Route::InstanceDetails;
                                }
                                cx.notify();
                            }))
                    })),
            )
        })
        .when(active == Route::VersionManager, |page| {
            let mut versions = div().flex().flex_col().gap(px(10.));
            versions = versions.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.muted)
                            .child(format!("{} installed · {} available", installed_versions.len(), version_releases.len())),
                    )
                    .child(
                        div()
                            .id("next-versions-refresh")
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(8.))
                            .bg(t.plate)
                            .text_color(t.ink)
                            .cursor_pointer()
                            .child(tr("Refresh"))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.fetch_releases();
                                cx.notify();
                            })),
                    ),
            );
            if let Some(error) = versions_error {
                versions = versions.child(
                    div()
                        .p(px(14.))
                        .rounded(px(9.))
                        .bg(t.plate_soft)
                        .text_color(t.danger)
                        .child(format!("{}: {error}", tr("Couldn't load releases."))),
                );
            } else if versions_loading {
                versions = versions.child(
                    div()
                        .p(px(18.))
                        .text_color(t.muted)
                        .child(tr("Loading releases…")),
                );
            } else if version_releases.is_empty() {
                versions = versions.child(
                    div()
                        .p(px(18.))
                        .text_color(t.muted)
                        .child(tr("No releases yet.")),
                );
            }
            versions = versions.children(version_releases.iter().map(|release| {
                let tag = release.tag.clone();
                let installed = installed_versions.contains(&tag);
                let selected = this.selected() == Some(&tag);
                let is_installing = installing_version.as_ref().is_some_and(|(current, _)| current == &tag);
                let is_removing = removing_versions.contains(&tag);
                let confirming = confirm_remove.as_deref() == Some(tag.as_str());
                let release_for_install = release.clone();
                let mut row = div()
                    .id(SharedString::from(format!("next-version-{tag}")))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .w_full()
                    .p(px(15.))
                    .rounded(px(10.))
                    .bg(if selected { t.surface } else { t.plate_soft })
                    .text_color(t.ink)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(px(4.))
                            .child(release.name.clone())
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(t.muted)
                                    .child(format!("{} · {}", tag, release.date)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(t.muted)
                                    .child(if selected {
                                        tr("In use")
                                    } else if installed {
                                        tr("Installed")
                                    } else {
                                        tr("Not installed")
                                    }),
                            ),
                    );
                if is_installing {
                    let percent = installing_version.as_ref().map(|(_, progress)| (*progress * 100.) as u32).unwrap_or(0);
                    row = row.child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.accent)
                            .child(format!("Installing · {percent}%")),
                    );
                } else if is_removing {
                    row = row.child(
                        div().text_size(px(12.)).text_color(t.muted).child(tr("Removing…")),
                    );
                } else if installed {
                    let use_tag = tag.clone();
                    let remove_tag = tag.clone();
                    let mut actions = div().flex().items_center().gap(px(6.));
                    if !selected && !confirming {
                        actions = actions.child(
                            div()
                                .id(SharedString::from(format!("next-version-use-{tag}")))
                                .px(px(10.))
                                .py(px(7.))
                                .rounded(px(7.))
                                .bg(t.plate)
                                .text_size(px(12.))
                                .cursor_pointer()
                                .child(tr("Use"))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.settings.selected = Some(use_tag.clone());
                                    this.save_settings();
                                    cx.notify();
                                })),
                        );
                    }
                    actions = actions.child(
                        div()
                            .id(SharedString::from(format!("next-version-remove-{tag}")))
                            .px(px(10.))
                            .py(px(7.))
                            .rounded(px(7.))
                            .bg(t.plate)
                            .text_size(px(12.))
                            .text_color(if confirming { t.danger } else { t.ink_soft })
                            .cursor_pointer()
                            .child(tr(if confirming { "Remove for good" } else { "Remove" }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if this.confirm_remove.as_deref() == Some(remove_tag.as_str()) {
                                    this.remove(&remove_tag);
                                } else {
                                    this.confirm_remove = Some(remove_tag.clone());
                                }
                                cx.notify();
                            })),
                    );
                    row = row.child(actions);
                } else if installing_version.is_some() {
                    row = row.child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.muted)
                            .child(tr("Another install is running")),
                    );
                } else if release.asset.is_some() {
                    row = row.child(
                        div()
                            .id(SharedString::from(format!("next-version-install-{tag}")))
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(7.))
                            .bg(t.accent)
                            .text_color(t.page)
                            .cursor_pointer()
                            .child(tr("Install"))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.install(&release_for_install);
                                cx.notify();
                            })),
                    );
                } else {
                    row = row.child(
                        div().text_size(px(11.)).text_color(t.muted).child(tr("No build for this platform")),
                    );
                }
                row
            }));
            if let Some((tag, error)) = version_error {
                versions = versions.child(
                    div()
                        .p(px(12.))
                        .rounded(px(8.))
                        .text_color(t.danger)
                        .bg(t.plate_soft)
                        .child(format!("{tag}: {error}")),
                );
            }
            page.child(versions)
        })
        .when(active == Route::InstanceDetails, |page| {
            page.child(instance_details_page.expect("instance details route has its page"))
        })
        .when(active == Route::AccountCenter, |page| {
            page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(accounts.iter().map(|account| {
                        let uuid = account.uuid.clone();
                        let provider = match horizon_launcher::next_facade::accounts::card(account).provider {
                            horizon_launcher::next_facade::accounts::Provider::Microsoft => tr("Microsoft"),
                            horizon_launcher::next_facade::accounts::Provider::ElyBy => tr("Ely.by"),
                            horizon_launcher::next_facade::accounts::Provider::Unavailable => tr("Provider unavailable"),
                        };
                        let state = auth_status
                            .get(&uuid)
                            .map(|status| format!("{:?}", status.state))
                            .unwrap_or_else(|| "Checking".into());
                        let selected = selected_account.as_deref() == Some(uuid.as_str());
                        div()
                            .id(SharedString::from(format!("next-account-{uuid}")))
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .w_full()
                            .p(px(16.))
                            .rounded(px(10.))
                            .bg(if selected { t.surface } else { t.plate_soft })
                            .text_color(t.ink)
                            .cursor_pointer()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(account.name.clone())
                                    .child(if selected { tr("Selected") } else { tr("Select account") }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(t.muted)
                                    .child(provider),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(t.muted)
                                    .child(format!("{}: {state}", tr("Session"))),
                            )
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.play_as(horizon_launcher::account::Player::Microsoft(uuid.clone()));
                                this.next_route = Route::AccountCenter;
                                cx.notify();
                            }))
                    }))
                    .children(offline_names.iter().map(|offline_name| {
                        let name = offline_name.clone();
                        let selected = offline_selected.as_deref() == Some(name.as_str());
                        div()
                            .id(SharedString::from(format!("next-offline-account-{name}")))
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .p(px(16.))
                            .rounded(px(10.))
                            .bg(if selected { t.surface } else { t.plate_soft })
                            .text_color(t.ink)
                            .cursor_pointer()
                            .child(name.clone())
                            .child(tr(if selected { "Selected · Offline" } else { "Select offline name" }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.play_as(horizon_launcher::account::Player::Offline(name.clone()));
                                this.next_route = Route::AccountCenter;
                                cx.notify();
                            }))
                    }))
                    .child(
                        div()
                            .id("next-game-generated-name")
                            .w_full()
                            .p(px(16.))
                            .rounded(px(10.))
                            .bg(if selected_account.is_none() && offline_selected.is_none() { t.surface } else { t.plate_soft })
                            .text_color(t.ink)
                            .cursor_pointer()
                            .child(tr("Use generated game name"))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.play_as(horizon_launcher::account::Player::GameName);
                                this.next_route = Route::AccountCenter;
                                cx.notify();
                            })),
                    )
                    .child(account_sign_in_panel.expect("account route has its sign-in panel")),
            )
        })
        .when(content_route, |page| {
            page.child(content_page.expect("content route has an existing workflow panel"))
        })
        .when(active == Route::ModWorkshop, |page| {
            page.child(native_workshop_page.expect("native workshop route has its own workflow"))
        })
        .when(
            !content_route
                && active != Route::Home
                && active != Route::ModWorkshop
                && active != Route::InstanceDetails,
            |page| {
            page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .p(px(24.))
                    .rounded(px(14.))
                    .bg(t.surface)
                    .child(
                        div()
                            .text_size(px(16.))
                            .text_color(t.ink)
                            .child(tr("This workspace is being migrated.")),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(t.muted)
                            .child(tr("The legacy workflow remains available during migration; this preview does not pretend that this panel is functional yet.")),
                    ),
            )
        });

    let mut root = div()
        .id("next-launcher-root")
        .flex()
        .size_full()
        .bg(t.page)
        .font_family(theme::FONT)
        .child(navigation)
        .child(page);
    // Preserve file-drop import, background build logs, account ownership gating and the
    // existing instance transaction while their new visual surfaces are migrated.
    root = this.import_drop_target(root, cx);
    if let Some(note) = this.import_note(cx) {
        root = root.child(note);
    }
    if let Some(dialog) = this.instance_dialog(cx) {
        root = root.child(dialog);
    }
    if let Some(panel) = this.native_log_panel(cx) {
        root = root.child(panel);
    }
    if this.asking_owner() {
        root = root.child(this.owner_screen(cx));
    }
    root
}

#[cfg(test)]
mod tests {
    use super::{layout_metrics, preview_enabled};

    #[test]
    fn shell_layout_scales_for_compact_and_standard_windows() {
        assert_eq!(layout_metrics(600.), (148., 14., 22.));
        assert_eq!(layout_metrics(900.), (184., 22., 26.));
        assert_eq!(layout_metrics(1200.), (232., 34., 30.));
    }

    #[test]
    fn workspace_parent_stays_highlighted_for_secondary_routes() {
        use horizon_launcher::next_navigation::{Route, Workspace};

        assert!(super::workspace_is_active(
            Workspace::Content,
            Route::InstalledContent
        ));
        assert!(super::workspace_is_active(
            Workspace::ModDevelopment,
            Route::ModWorkshop
        ));
        assert!(!super::workspace_is_active(
            Workspace::Content,
            Route::ServerConsole
        ));
    }

    #[test]
    fn content_routes_select_the_correct_existing_package_workflow() {
        use horizon_launcher::next_navigation::Route;

        assert_eq!(
            super::content_kind(Route::ContentBrowser),
            Some(horizon_launcher::browser::Kind::Mod)
        );
        assert_eq!(
            super::content_kind(Route::InstalledContent),
            Some(horizon_launcher::browser::Kind::Mod)
        );
        assert_eq!(super::content_kind(Route::ModWorkshop), None);
        assert_eq!(
            super::content_kind(Route::ModpackManager),
            Some(horizon_launcher::browser::Kind::Modpack)
        );
        assert_eq!(super::content_kind(Route::ServerDashboard), None);
    }

    #[test]
    fn preview_requires_explicit_opt_in() {
        assert!(preview_enabled(Some("1")));
        for value in [None, Some(""), Some("0"), Some("true"), Some("yes")] {
            assert!(!preview_enabled(value));
        }
    }
}

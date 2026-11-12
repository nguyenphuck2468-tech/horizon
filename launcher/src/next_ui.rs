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
    let auth_status = this.auth_status.clone();
    let selected_version = this.selected().cloned();
    let game_running = this.client.is_some() || this.starting_client.is_some();

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
                            this.next_route = primary;
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
                                    this.next_route = route;
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
                            .text_size(px(12.))
                            .text_color(t.muted)
                            .child(tr("Play delegates to Horizon's existing owner, session, preparation and launch checks.")),
                    ),
            )
        })
        .when(active == Route::InstanceLibrary, |page| {
            page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(instances.iter().map(|instance| {
                        let id = instance.id.clone();
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
                            .child(tr(if selected { "Selected" } else { "Select" }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if let Some(chosen) = this.instances.list.iter().find(|item| item.id == id).cloned() {
                                    this.select_instance(&chosen);
                                    this.next_route = Route::Home;
                                }
                                cx.notify();
                            }))
                    })),
            )
        })
        .when(active == Route::AccountCenter, |page| {
            page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(accounts.iter().map(|account| {
                        let state = auth_status
                            .get(&account.uuid)
                            .map(|status| format!("{:?}", status.state))
                            .unwrap_or_else(|| "Checking".into());
                        let selected = selected_account.as_deref() == Some(account.uuid.as_str());
                        div()
                            .id(SharedString::from(format!("next-account-{}", account.uuid)))
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .w_full()
                            .p(px(16.))
                            .rounded(px(10.))
                            .bg(t.surface)
                            .text_color(t.ink)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(account.name.clone())
                                    .child(if selected { "Selected" } else { "" }),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(t.muted)
                                    .child(format!("{}: {state}", tr("Session"))),
                            )
                    })),
            )
        })
        .when(active == Route::ModWorkshop, |page| {
            page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .p(px(24.))
                    .rounded(px(14.))
                    .bg(t.surface)
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(t.ink)
                            .child(tr("Native mod build workspace")),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(t.ink_soft)
                            .child(tr("The existing native build pipeline is now exposed through the typed next_facade boundary. The next screen step wires bundle discovery, profile selection, cancellable background execution and build logs.")),
                    ),
            )
        })
        .when(active != Route::Home && active != Route::ModWorkshop, |page| {
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

    div()
        .flex()
        .size_full()
        .bg(t.page)
        .font_family(theme::FONT)
        .child(navigation)
        .child(page)
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
    fn preview_requires_explicit_opt_in() {
        assert!(preview_enabled(Some("1")));
        for value in [None, Some(""), Some("0"), Some("true"), Some("yes")] {
            assert!(!preview_enabled(value));
        }
    }
}

//! First native GPUI shell for the replacement launcher frontend.
//!
//! This is deliberately a new screen tree, not a reskin of the legacy pages. It starts as a
//! navigation/layout preview while feature workflows are migrated through next_facade. Set
//! HORIZON_LAUNCHER_NEXT_UI=1 in a development build to open it. The final release cutover
//! removes this preview gate and the old renderer after feature-parity checks pass.

use super::*;
use gpui::{div, prelude::*, px, ClickEvent, SharedString};

/// Whether the replacement shell is requested for local development and screenshot tests.
fn preview_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Whether the replacement shell is requested for local development and screenshot tests.
/// Require an explicit value so unrelated inherited environment variables cannot switch UIs.
pub(super) fn wanted() -> bool {
    preview_enabled(std::env::var("HORIZON_LAUNCHER_NEXT_UI").ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::preview_enabled;

    #[test]
    fn preview_requires_explicit_opt_in() {
        assert!(preview_enabled(Some("1")));
        for value in [None, Some(""), Some("0"), Some("true"), Some("yes")] {
            assert!(!preview_enabled(value));
        }
    }
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
    let active = this.next_route;
    let instance_name = this
        .selected_instance()
        .map(|instance| instance.name.clone())
        .unwrap_or_else(|| "No instance selected".into());

    let navigation = div()
        .flex()
        .flex_col()
        .w(px(232.))
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
                .child("NEXT-GENERATION LAUNCHER"),
        )
        .children(ROUTES.iter().map(|entry| {
            let route = entry.route;
            let selected = route == active;
            div()
                .id(SharedString::from(format!("next-route-{}", entry.id)))
                .w_full()
                .px(px(12.))
                .py(px(10.))
                .rounded(px(8.))
                .text_size(px(13.))
                .text_color(if selected { t.accent } else { t.ink_soft })
                .when(selected, |element| element.bg(t.surface))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.next_route = route;
                    cx.notify();
                }))
                .child(label(route))
        }));

    let page = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .h_full()
        .p(px(34.))
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
                                .text_size(px(30.))
                                .text_color(t.ink)
                                .child(label(active)),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(t.muted)
                                .child(description(active)),
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
                        .child("Preview · feature migration in progress"),
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
                            .child("SELECTED INSTANCE"),
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
                            .child("The new Home workflow will connect instance readiness, account status, Java and Play."),
                    ),
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
                            .child("Native mod build workspace"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(t.ink_soft)
                            .child("The existing native build pipeline is now exposed through the typed next_facade boundary. The next screen step wires bundle discovery, profile selection, cancellable background execution and build logs."),
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
                            .child("This workspace is being migrated."),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(t.muted)
                            .child("The legacy workflow remains available during migration; this preview does not pretend that this panel is functional yet."),
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

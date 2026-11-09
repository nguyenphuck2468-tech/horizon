//! Stable route and feature metadata for the next-generation launcher shell.
//!
//! This module intentionally contains no GPUI elements or business logic. The presentation
//! layer resolves these IDs to localized labels and views; application services remain elsewhere.

/// A top-level workspace in the next-generation launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Workspace {
    Home,
    Instances,
    Accounts,
    Content,
    Servers,
    Multiplayer,
    Worlds,
    Cosmetics,
    Diagnostics,
    Settings,
}

/// A stable destination inside a workspace. IDs are internal route identifiers, not localized
/// strings, and must not be persisted as user data without a migration contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    InstanceLibrary,
    InstanceDetails,
    VersionManager,
    AccountCenter,
    AccountDetails,
    ContentBrowser,
    InstalledContent,
    ModpackManager,
    ServerDashboard,
    ServerConsole,
    MultiplayerBrowser,
    WorldLibrary,
    ScreenshotLibrary,
    Wardrobe,
    DiagnosticsOverview,
    DownloadQueue,
    CrashReports,
    GlobalSettings,
    InstanceSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteMetadata {
    pub route: Route,
    pub workspace: Workspace,
    /// Translation key; never render this directly as user-facing text.
    pub title_key: &'static str,
    /// Stable machine-readable identifier for logs and navigation tests.
    pub id: &'static str,
    /// Whether this destination is a canonical management surface.
    pub canonical: bool,
}

/// Canonical route catalogue. Contextual shortcuts should navigate here rather than create a
/// second, competing management surface for the same feature.
pub const ROUTES: &[RouteMetadata] = &[
    RouteMetadata { route: Route::Home, workspace: Workspace::Home, title_key: "nav.home", id: "home", canonical: true },
    RouteMetadata { route: Route::InstanceLibrary, workspace: Workspace::Instances, title_key: "nav.instances", id: "instances.library", canonical: true },
    RouteMetadata { route: Route::InstanceDetails, workspace: Workspace::Instances, title_key: "nav.instance_details", id: "instances.details", canonical: true },
    RouteMetadata { route: Route::VersionManager, workspace: Workspace::Instances, title_key: "nav.versions", id: "instances.versions", canonical: true },
    RouteMetadata { route: Route::AccountCenter, workspace: Workspace::Accounts, title_key: "nav.accounts", id: "accounts.center", canonical: true },
    RouteMetadata { route: Route::AccountDetails, workspace: Workspace::Accounts, title_key: "nav.account_details", id: "accounts.details", canonical: true },
    RouteMetadata { route: Route::ContentBrowser, workspace: Workspace::Content, title_key: "nav.content", id: "content.browser", canonical: true },
    RouteMetadata { route: Route::InstalledContent, workspace: Workspace::Content, title_key: "nav.installed_content", id: "content.installed", canonical: true },
    RouteMetadata { route: Route::ModpackManager, workspace: Workspace::Content, title_key: "nav.modpacks", id: "content.modpacks", canonical: true },
    RouteMetadata { route: Route::ServerDashboard, workspace: Workspace::Servers, title_key: "nav.servers", id: "servers.dashboard", canonical: true },
    RouteMetadata { route: Route::ServerConsole, workspace: Workspace::Servers, title_key: "nav.server_console", id: "servers.console", canonical: true },
    RouteMetadata { route: Route::MultiplayerBrowser, workspace: Workspace::Multiplayer, title_key: "nav.multiplayer", id: "multiplayer.browser", canonical: true },
    RouteMetadata { route: Route::WorldLibrary, workspace: Workspace::Worlds, title_key: "nav.worlds", id: "worlds.library", canonical: true },
    RouteMetadata { route: Route::ScreenshotLibrary, workspace: Workspace::Worlds, title_key: "nav.screenshots", id: "worlds.screenshots", canonical: true },
    RouteMetadata { route: Route::Wardrobe, workspace: Workspace::Cosmetics, title_key: "nav.wardrobe", id: "cosmetics.wardrobe", canonical: true },
    RouteMetadata { route: Route::DiagnosticsOverview, workspace: Workspace::Diagnostics, title_key: "nav.diagnostics", id: "diagnostics.overview", canonical: true },
    RouteMetadata { route: Route::DownloadQueue, workspace: Workspace::Diagnostics, title_key: "nav.downloads", id: "diagnostics.downloads", canonical: true },
    RouteMetadata { route: Route::CrashReports, workspace: Workspace::Diagnostics, title_key: "nav.crash_reports", id: "diagnostics.crashes", canonical: true },
    RouteMetadata { route: Route::GlobalSettings, workspace: Workspace::Settings, title_key: "nav.settings", id: "settings.global", canonical: true },
    RouteMetadata { route: Route::InstanceSettings, workspace: Workspace::Settings, title_key: "nav.instance_settings", id: "settings.instance", canonical: true },
];

/// Functional areas that must remain reachable while the shell is replaced. This list is used by
/// preservation tests and is intentionally broader than the visible primary navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureArea {
    AccountsAndIdentity,
    VersionsAndBuilds,
    InstallPrepareLaunch,
    Instances,
    ModsAndPacks,
    WorldsAndScreenshots,
    Multiplayer,
    ServersAndNetwork,
    JavaAndLaunchSettings,
    ConsoleAndDiagnostics,
    SkinsAndCosmetics,
    ConfigurationAndPlatform,
}

pub const PRESERVED_FEATURE_AREAS: &[FeatureArea] = &[
    FeatureArea::AccountsAndIdentity,
    FeatureArea::VersionsAndBuilds,
    FeatureArea::InstallPrepareLaunch,
    FeatureArea::Instances,
    FeatureArea::ModsAndPacks,
    FeatureArea::WorldsAndScreenshots,
    FeatureArea::Multiplayer,
    FeatureArea::ServersAndNetwork,
    FeatureArea::JavaAndLaunchSettings,
    FeatureArea::ConsoleAndDiagnostics,
    FeatureArea::SkinsAndCosmetics,
    FeatureArea::ConfigurationAndPlatform,
];

/// Find a route by its stable ID.
pub fn route_by_id(id: &str) -> Option<&'static RouteMetadata> {
    ROUTES.iter().find(|entry| entry.id == id)
}

/// Find the canonical workspace for a route.
pub fn workspace_for(route: Route) -> Workspace {
    ROUTES
        .iter()
        .find(|entry| entry.route == route)
        .expect("every Route variant must have metadata")
        .workspace
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn route_ids_and_route_variants_are_unique() {
        let mut ids = HashSet::new();
        let mut routes = HashSet::new();
        for entry in ROUTES {
            assert!(ids.insert(entry.id), "duplicate route ID: {}", entry.id);
            assert!(routes.insert(entry.route), "duplicate route: {:?}", entry.route);
            assert!(entry.canonical, "route must have one canonical destination: {}", entry.id);
            assert!(!entry.title_key.is_empty());
        }
        assert_eq!(routes.len(), 20);
    }

    #[test]
    fn every_workspace_has_a_canonical_destination() {
        for workspace in [
            Workspace::Home,
            Workspace::Instances,
            Workspace::Accounts,
            Workspace::Content,
            Workspace::Servers,
            Workspace::Multiplayer,
            Workspace::Worlds,
            Workspace::Cosmetics,
            Workspace::Diagnostics,
            Workspace::Settings,
        ] {
            assert!(
                ROUTES.iter().any(|entry| entry.workspace == workspace && entry.canonical),
                "workspace has no canonical route: {:?}",
                workspace
            );
        }
    }

    #[test]
    fn stable_route_lookup_and_workspace_resolution_work() {
        assert_eq!(route_by_id("accounts.center").unwrap().route, Route::AccountCenter);
        assert_eq!(route_by_id("content.modpacks").unwrap().workspace, Workspace::Content);
        assert!(route_by_id("not-a-route").is_none());
        assert_eq!(workspace_for(Route::DownloadQueue), Workspace::Diagnostics);
    }

    #[test]
    fn the_full_legacy_feature_inventory_remains_in_the_parity_contract() {
        assert_eq!(PRESERVED_FEATURE_AREAS.len(), 12);
        assert!(PRESERVED_FEATURE_AREAS.contains(&FeatureArea::AccountsAndIdentity));
        assert!(PRESERVED_FEATURE_AREAS.contains(&FeatureArea::InstallPrepareLaunch));
        assert!(PRESERVED_FEATURE_AREAS.contains(&FeatureArea::ModsAndPacks));
        assert!(PRESERVED_FEATURE_AREAS.contains(&FeatureArea::ServersAndNetwork));
        assert!(PRESERVED_FEATURE_AREAS.contains(&FeatureArea::ConfigurationAndPlatform));
    }
}

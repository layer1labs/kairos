use super::{
    settings_page::{
        render_body_item, MatchData, PageType, SettingsPageEvent, SettingsPageMeta,
        SettingsPageViewHandle, SettingsWidget,
    },
    LocalOnlyIconState, SettingsSection, ToggleState,
};
use crate::{
    appearance::Appearance,
    channel::ChannelState,
    kairos_updater::{KairosUpdateChannel, KairosUpdateStatus, KairosUpdaterEvent, KairosUpdaterState},
    report_if_error,
    settings::AutoupdateSettings,
    workspace::WorkspaceAction,
};
use settings::Setting as _;
use warp_core::settings::ToggleableSetting as _;
use warpui::ui_components::{
    button::ButtonVariant,
    components::{Coords, UiComponent, UiComponentStyles},
    switch::SwitchStateHandle,
};
use warpui::{
    assets::asset_cache::AssetSource,
    elements::{
        Align, CacheOption, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Image,
        MainAxisAlignment, MouseStateHandle, ParentElement, Wrap,
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

/// Computes the copyright string with a dynamic year range.
/// - If the current year is 2026: "Copyright 2026 BitConcepts, LLC."
/// - If the current year is greater: "Copyright 2026 \u{2013} YYYY BitConcepts, LLC."
fn kairos_copyright() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Approximate current year: seconds_since_epoch / seconds_per_year + 1970
    let year = (secs / 31_557_600 + 1970) as u32;
    if year <= 2026 {
        "Copyright 2026 BitConcepts, LLC.".to_string()
    } else {
        format!("Copyright 2026 \u{2013} {year} BitConcepts, LLC.")
    }
}

#[derive(Debug, Clone)]
pub enum AboutPageAction {
    ToggleAutomaticUpdates,
    /// Switch the Kairos update channel.
    SetUpdateChannel(KairosUpdateChannel),
    /// Trigger an on-demand update check.
    CheckForUpdates,
    /// Open a release page URL in the system browser.
    OpenReleasePage(String),
}

pub struct AboutPageView {
    page: PageType<Self>,
}

impl AboutPageView {
    pub fn new(ctx: &mut ViewContext<AboutPageView>) -> Self {
        // Re-render whenever the updater state changes (channel loaded, check
        // completes, etc.).
        let updater_handle = KairosUpdaterState::handle(ctx);
        ctx.subscribe_to_model(
            &updater_handle,
            |_me, _handle, _event: &KairosUpdaterEvent, ctx| {
                ctx.notify();
            },
        );
        AboutPageView {
            page: PageType::new_monolith(AboutPageWidget::default(), None, false),
        }
    }
}

impl Entity for AboutPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for AboutPageView {
    type Action = AboutPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            AboutPageAction::ToggleAutomaticUpdates => {
                AutoupdateSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings
                        .automatic_updates_enabled
                        .toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            AboutPageAction::SetUpdateChannel(channel) => {
                let channel = *channel;
                KairosUpdaterState::handle(ctx).update(ctx, |state, ctx| {
                    state.set_channel(channel, ctx);
                });
                ctx.notify();
            }
            AboutPageAction::CheckForUpdates => {
                KairosUpdaterState::handle(ctx).update(ctx, |state, ctx| {
                    state.check_for_update(ctx);
                });
                ctx.notify();
            }
            AboutPageAction::OpenReleasePage(url) => {
                ctx.open_url(url);
            }
        }
    }
}

impl View for AboutPageView {
    fn ui_name() -> &'static str {
        "AboutPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Default)]
struct AboutPageWidget {
    copy_version_button_mouse_state: MouseStateHandle,
    automatic_updates_switch_state: SwitchStateHandle,
    /// Button handles for the channel selector and update actions.
    stable_pill_button: MouseStateHandle,
    latest_pill_button: MouseStateHandle,
    check_now_button: MouseStateHandle,
    open_release_button: MouseStateHandle,
}

impl SettingsWidget for AboutPageWidget {
    type View = AboutPageView;

    fn search_terms(&self) -> &str {
        "about kairos version automatic updates auto update 自动更新"
    }

    fn render(
        &self,
        _view: &AboutPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();

        let image_path = "bundled/svg/kairos-wordmark.svg";

        // GIT_RELEASE_TAG env var injected at build time → shows tag; otherwise falls back to "Dev".
        let version = ChannelState::app_version().unwrap_or("Dev");

        let version_text = ui_builder
            .span(version.to_string())
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let copy_version_icon = appearance
            .ui_builder()
            .copy_button(16., self.copy_version_button_mouse_state.clone())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(version));
            })
            .finish();

        let version_row = Wrap::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_children([
                version_text,
                Container::new(copy_version_icon)
                    .with_margin_top(16.)
                    .with_padding_left(6.)
                    .finish(),
            ]);

        let mut content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ConstrainedBox::new(
                    Image::new(
                        AssetSource::Bundled { path: image_path },
                        CacheOption::BySize,
                    )
                    .finish(),
                )
                .with_max_height(100.)
                .with_max_width(350.)
                .finish(),
            )
            .with_child(
                ui_builder
                    .span("Kairos")
                    .build()
                    .with_margin_top(12.)
                    .finish(),
            )
            .with_child(version_row.finish())
            .with_child(
                ui_builder
                    .span(kairos_copyright())
                    .build()
                    .with_margin_top(16.)
                    .finish(),
            );

        // ── Automatic updates toggle (now live) ──────────────────────────────
        let auto_updates_on =
            *AutoupdateSettings::as_ref(app).automatic_updates_enabled.value();
        content.add_child(
            Container::new(
                ConstrainedBox::new(render_body_item::<AboutPageAction>(
                    crate::t!("settings-about-automatic-updates-label"),
                    None,
                    LocalOnlyIconState::Hidden,
                    ToggleState::Enabled,
                    appearance,
                    appearance
                        .ui_builder()
                        .switch(self.automatic_updates_switch_state.clone())
                        .check(auto_updates_on)
                        .build()
                        .on_click(|ctx, _, _| {
                            ctx.dispatch_typed_action(AboutPageAction::ToggleAutomaticUpdates);
                        })
                        .finish(),
                    Some(crate::t!("settings-about-automatic-updates-description")),
                ))
                .with_max_width(520.)
                .finish(),
            )
            .with_margin_top(24.)
            .finish(),
        );

        // ── Update channel selector ──────────────────────────────────────────
        let (active_channel, update_status) = {
            let updater = KairosUpdaterState::as_ref(app);
            (updater.channel, updater.status.clone())
        };

        // Small style for pill buttons.
        let pill_style = UiComponentStyles {
            font_size: Some(12.),
            padding: Some(Coords::uniform(6.)),
            ..Default::default()
        };

        // Stable pill: highlighted (Accent) if active, secondary button if inactive.
        let stable_pill: Box<dyn Element> = if active_channel == KairosUpdateChannel::Stable {
            appearance
                .ui_builder()
                .button(ButtonVariant::Accent, self.stable_pill_button.clone())
                .with_style(pill_style.clone())
                .with_centered_text_label(KairosUpdateChannel::Stable.label().to_string())
                .build()
                .finish()
        } else {
            appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, self.stable_pill_button.clone())
                .with_style(pill_style.clone())
                .with_centered_text_label(KairosUpdateChannel::Stable.label().to_string())
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(AboutPageAction::SetUpdateChannel(
                        KairosUpdateChannel::Stable,
                    ));
                })
                .finish()
        };

        // Latest pill: similar pattern.
        let latest_pill: Box<dyn Element> = if active_channel == KairosUpdateChannel::Latest {
            appearance
                .ui_builder()
                .button(ButtonVariant::Accent, self.latest_pill_button.clone())
                .with_style(pill_style.clone())
                .with_centered_text_label(KairosUpdateChannel::Latest.label().to_string())
                .build()
                .finish()
        } else {
            appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, self.latest_pill_button.clone())
                .with_style(pill_style.clone())
                .with_centered_text_label(KairosUpdateChannel::Latest.label().to_string())
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(AboutPageAction::SetUpdateChannel(
                        KairosUpdateChannel::Latest,
                    ));
                })
                .finish()
        };

        let channel_row = Wrap::row()
            .with_children([
                appearance
                    .ui_builder()
                    .span(crate::t!("settings-about-update-channel-label"))
                    .build()
                    .finish(),
                Container::new(stable_pill).with_padding_left(8.).finish(),
                Container::new(latest_pill).with_padding_left(6.).finish(),
            ])
            .finish();

        content.add_child(
            Container::new(channel_row)
                .with_margin_top(16.)
                .finish(),
        );

        // ── Update status row ────────────────────────────────────────────────
        let status_text: String = match &update_status {
            KairosUpdateStatus::Idle => crate::t!("settings-about-update-status-idle").into(),
            KairosUpdateStatus::Checking =>
                crate::t!("settings-about-update-status-checking").into(),
            KairosUpdateStatus::UpToDate =>
                crate::t!("settings-about-update-status-up-to-date").into(),
            KairosUpdateStatus::Available { version, .. } => {
                format!("v{} available", version)
            }
            KairosUpdateStatus::Error(msg) => format!("Error: {}", msg),
        };

        let status_label = appearance
            .ui_builder()
            .span(status_text)
            .build()
            .finish();

        // "Check Now" button (secondary action button).
        let check_button_style = UiComponentStyles {
            font_size: Some(12.),
            padding: Some(Coords::uniform(6.)),
            ..Default::default()
        };
        let check_button = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, self.check_now_button.clone())
            .with_style(check_button_style.clone())
            .with_centered_text_label(
                crate::t!("settings-about-check-for-updates").to_string(),
            )
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(AboutPageAction::CheckForUpdates);
            })
            .finish();

        // If an update is available, show an "Open release page" button.
        let open_link: Option<Box<dyn Element>> =
            if let KairosUpdateStatus::Available { html_url, .. } = &update_status {
                let url = html_url.clone();
                Some(
                    appearance
                        .ui_builder()
                        .button(ButtonVariant::Secondary, self.open_release_button.clone())
                        .with_style(check_button_style)
                        .with_centered_text_label(
                            crate::t!("settings-about-open-release").to_string(),
                        )
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(AboutPageAction::OpenReleasePage(
                                url.clone(),
                            ));
                        })
                        .finish(),
                )
            } else {
                None
            };

        let mut status_row = Wrap::row().with_children([status_label, check_button]);
        if let Some(link) = open_link {
            status_row.add_child(
                Container::new(link).with_padding_left(8.).finish(),
            );
        }

        content.add_child(
            Container::new(status_row.finish())
                .with_margin_top(8.)
                .finish(),
        );

        Align::new(content.finish()).finish()
    }
}

impl SettingsPageMeta for AboutPageView {
    fn section() -> SettingsSection {
        SettingsSection::About
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<AboutPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AboutPageView>) -> Self {
        SettingsPageViewHandle::About(view_handle)
    }
}

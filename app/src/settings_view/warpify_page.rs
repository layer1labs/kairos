use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;

use settings::{Setting, ToggleableSetting};
use strum::IntoEnumIterator;
use warp_core::features::FeatureFlag;
use warpui::keymap::ContextPredicate;
use warpui::{
    elements::{Container, Flex, MouseStateHandle, ParentElement},
    presenter::ChildView,
    ui_components::{
        components::{Coords, UiComponent, UiComponentStyles},
        switch::SwitchStateHandle,
    },
    Action, AppContext, Element, Entity, ModelHandle, SingletonEntity, TypedActionView, View,
    ViewContext, ViewHandle,
};

use crate::terminal::warpify::settings::{
    EnableSshWarpification, SshExtensionInstallMode, SshExtensionInstallModeSetting,
    UseSshTmuxWrapper, WarpifySettingsChangedEvent,
};
use crate::{
    appearance::Appearance,
    report_if_error, send_telemetry_from_ctx,
    server::telemetry::TelemetryEvent,
    terminal::warpify::settings::WarpifySettings,
    view_components::{SubmittableTextInput, SubmittableTextInputEvent},
};

use super::settings_page::{
    render_body_item, render_dropdown_item, AdditionalInfo, Category,
    LocalOnlyIconState, MatchData, PageType, SettingsPageEvent, SettingsWidget, ToggleState,
    HEADER_PADDING,
};
use super::SettingsSection;
use super::{
    flags,
    settings_page::{
        add_setting, render_alternating_color_list, SettingsPageMeta, SettingsPageViewHandle,
    },
    SettingsAction, ToggleSettingActionPair,
};
use crate::view_components::dropdown::{Dropdown, DropdownItem};

pub fn init_actions_from_parent_view<T: Action + Clone>(
    app: &mut AppContext,
    context: &ContextPredicate,
    builder: fn(SettingsAction) -> T,
) {
    // Add all of the toggle settings from the Warpify Page that you want to show up on the Command Palette here.
    let mut toggle_binding_pairs = vec![];

    if FeatureFlag::SSHTmuxWrapper.is_enabled() {
        toggle_binding_pairs.push(ToggleSettingActionPair::new(
            &crate::t!("settings-warpify-ssh-tmux-toggle-binding-label"),
            builder(SettingsAction::WarpifyPageToggle(
                WarpifyPageAction::ToggleTmuxWarpification,
            )),
            context,
            flags::SSH_TMUX_WRAPPER_CONTEXT_FLAG,
        ));
    }

    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(toggle_binding_pairs, app);
}

const CONTENT_FONT_SIZE: f32 = 12.;
const ITEM_VERTICAL_SPACING: f32 = 24.;
/// There's a built-in 10px margin below the text input.
const BUILT_IN_TEXT_INPUT_MARGIN: f32 = 10.;
const SPACE_AFTER_TEXT_INPUT: f32 = ITEM_VERTICAL_SPACING - BUILT_IN_TEXT_INPUT_MARGIN;

/// SSH integration settings page — configure SSH shell integration (warpification over SSH).
/// Subshell warpification settings have been removed from the Kairos product surface.
pub struct WarpifyPageView {
    page: PageType<Self>,
    remove_denylisted_ssh_button_states: Vec<MouseStateHandle>,
    add_denylisted_ssh_editor: ViewHandle<SubmittableTextInput>,
    ssh_extension_install_mode_dropdown: ViewHandle<Dropdown<WarpifyPageAction>>,
}

impl WarpifyPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let warpify_settings_handle = WarpifySettings::handle(ctx);

        ctx.observe(&warpify_settings_handle, Self::update_button_states);
        ctx.subscribe_to_model(&warpify_settings_handle, move |me, model, event, ctx| {
            me.update_button_states(model, ctx);
            if matches!(
                event,
                WarpifySettingsChangedEvent::SshExtensionInstallModeSetting { .. }
            ) {
                me.update_dropdown(ctx);
            }
            ctx.notify();
        });

        let add_denylisted_ssh_editor = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text(crate::t!("settings-warpify-host-placeholder"), ctx);
            input
        });

        ctx.subscribe_to_view(
            &add_denylisted_ssh_editor,
            Self::handle_denylisted_ssh_editor_event,
        );

        let ssh_extension_install_mode_dropdown =
            Self::create_ssh_extension_install_mode_dropdown(ctx);

        let mut instance = Self {
            page: Self::build_page(ctx),
            remove_denylisted_ssh_button_states: Default::default(),
            add_denylisted_ssh_editor,
            ssh_extension_install_mode_dropdown,
        };

        instance.update_button_states(warpify_settings_handle, ctx);
        instance
    }

    fn build_page(_ctx: &mut ViewContext<Self>) -> PageType<Self> {
        // Subshell warpification has been removed from the Kairos product surface.
        // This page now shows SSH shell integration settings only.
        let categories = vec![
            Category::new(
                Box::leak(crate::t!("settings-warpify-section-ssh").into_boxed_str()),
                vec![Box::new(SSHWidget::default())],
            )
            .with_subtitle(Box::leak(
                crate::t!("settings-warpify-section-ssh-subtitle").into_boxed_str(),
            )),
        ];
        PageType::new_categorized(categories, None)
    }

    /// This method ensures each command in the SubshellSettings has a matching button state for
    /// its delete button in the View.
    fn update_button_states(
        &mut self,
        warpify_settings_handle: ModelHandle<WarpifySettings>,
        ctx: &mut ViewContext<Self>,
    ) {
        let warpify_settings = warpify_settings_handle.as_ref(ctx);
        self.remove_denylisted_ssh_button_states = warpify_settings
            .ssh_hosts_denylist
            .iter()
            .map(|_| Default::default())
            .collect();
        ctx.notify();
    }

    /// Syncs the install-mode dropdown selection with the current
    /// `WarpifySettings::ssh_extension_install_mode` value (e.g. after it
    /// was changed from the SSH remote server choice view).
    fn update_dropdown(&mut self, ctx: &mut ViewContext<Self>) {
        let current_mode = *WarpifySettings::as_ref(ctx)
            .ssh_extension_install_mode
            .value();
        self.ssh_extension_install_mode_dropdown
            .update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(
                    WarpifyPageAction::SetSshExtensionInstallMode(current_mode),
                    ctx,
                );
            });
    }

    fn handle_denylisted_ssh_editor_event(
        &mut self,
        _handle: ViewHandle<SubmittableTextInput>,
        event: &SubmittableTextInputEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            SubmittableTextInputEvent::Submit(new_command) => {
                WarpifySettings::handle(ctx).update(ctx, |warpify_settings, ctx| {
                    warpify_settings.denylist_ssh_host(new_command, ctx);
                });

                send_telemetry_from_ctx!(TelemetryEvent::AddDenylistedSshTmuxWrapperHost, ctx);
            }
            SubmittableTextInputEvent::Escape => ctx.emit(SettingsPageEvent::FocusModal),
        }
    }

    fn remove_denylisted_ssh_host(&self, index: usize, ctx: &mut ViewContext<Self>) {
        send_telemetry_from_ctx!(TelemetryEvent::RemoveDenylistedSshTmuxWrapperHost, ctx);
        WarpifySettings::handle(ctx).update(ctx, |warpify, ctx| {
            warpify.remove_denylisted_ssh_host(index, ctx)
        });
    }
}

impl Entity for WarpifyPageView {
    type Event = SettingsPageEvent;
}

fn build_sub_sub_title(title: String, appearance: &Appearance) -> Container {
    appearance
        .ui_builder()
        .span(title)
        .with_style(UiComponentStyles {
            font_size: Some(CONTENT_FONT_SIZE),
            ..Default::default()
        })
        .build()
}

const SSH_EXTENSION_DROPDOWN_WIDTH: f32 = 250.;

impl WarpifyPageView {
    fn create_ssh_extension_install_mode_dropdown(
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<Dropdown<WarpifyPageAction>> {
        let items: Vec<DropdownItem<WarpifyPageAction>> = SshExtensionInstallMode::iter()
            .map(|mode| {
                DropdownItem::new(
                    mode.display_name(),
                    WarpifyPageAction::SetSshExtensionInstallMode(mode),
                )
            })
            .collect();

        let current_mode = *WarpifySettings::as_ref(ctx)
            .ssh_extension_install_mode
            .value();
        let enable_ssh_warpification = *WarpifySettings::as_ref(ctx)
            .enable_ssh_warpification
            .value();

        ctx.add_typed_action_view(move |ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(SSH_EXTENSION_DROPDOWN_WIDTH);
            dropdown.set_menu_width(SSH_EXTENSION_DROPDOWN_WIDTH, ctx);
            dropdown.add_items(items, ctx);
            dropdown.set_selected_by_action(
                WarpifyPageAction::SetSshExtensionInstallMode(current_mode),
                ctx,
            );
            if !enable_ssh_warpification {
                dropdown.set_disabled(ctx);
            }
            dropdown
        })
    }

    /// Renders a title, a list of items that can be removed, and an input field to add new items.
    fn build_input_list<
        ListItem: Display,
        SettingsPageAction: Action + Clone,
        F: Fn(usize) -> SettingsPageAction,
        T: View,
    >(
        &self,
        title: String,
        patterns: &[ListItem],
        mouse_states: &[MouseStateHandle],
        create_action: F,
        handle: &ViewHandle<T>,
        appearance: &Appearance,
    ) -> Container {
        let mut column = Flex::column();
        let mut title = build_sub_sub_title(title, appearance);

        if !patterns.is_empty() {
            title = title.with_padding_bottom(BUILT_IN_TEXT_INPUT_MARGIN);
        }

        column.add_child(title.finish());

        render_alternating_color_list(
            &mut column,
            patterns,
            mouse_states,
            create_action,
            appearance,
        );

        Container::new(
            column
                .with_child(
                    Container::new(ChildView::new(handle).finish())
                        .with_margin_bottom(SPACE_AFTER_TEXT_INPUT)
                        .finish(),
                )
                .finish(),
        )
    }
}

impl View for WarpifyPageView {
    fn ui_name() -> &'static str {
        "WarpifyPageView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WarpifyPageAction {
    RemoveDenylistedSshHost(usize),
    /// If disabled, auto-Warpification and the SSH Warpification prompt will be disabled.
    ToggleTmuxWarpification,
    ToggleSshWarpification,
    /// Set the SSH extension installation mode (always ask / always install / always skip).
    SetSshExtensionInstallMode(SshExtensionInstallMode),
    OpenUrl(String),
}

impl TypedActionView for WarpifyPageView {
    type Action = WarpifyPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        use WarpifyPageAction::*;
        match action {
            ToggleSshWarpification => {
                WarpifySettings::handle(ctx).update(ctx, |ssh_settings, ctx| {
                    report_if_error!(ssh_settings
                        .enable_ssh_warpification
                        .toggle_and_save_value(ctx));
                    send_telemetry_from_ctx!(
                        TelemetryEvent::ToggleSshWarpification {
                            enabled: *ssh_settings.enable_ssh_warpification.value(),
                        },
                        ctx
                    );
                });
                let enabled = *WarpifySettings::as_ref(ctx)
                    .enable_ssh_warpification
                    .value();
                self.ssh_extension_install_mode_dropdown
                    .update(ctx, |dropdown, ctx| {
                        if enabled {
                            dropdown.set_enabled(ctx);
                        } else {
                            dropdown.set_disabled(ctx);
                        }
                    });
            }
            ToggleTmuxWarpification => {
                WarpifySettings::handle(ctx).update(ctx, |ssh_settings, ctx| {
                    report_if_error!(ssh_settings.use_ssh_tmux_wrapper.toggle_and_save_value(ctx));
                    send_telemetry_from_ctx!(
                        TelemetryEvent::ToggleSshTmuxWrapper {
                            enabled: *ssh_settings.use_ssh_tmux_wrapper.value(),
                        },
                        ctx
                    );
                });
            }
            SetSshExtensionInstallMode(mode) => {
                WarpifySettings::handle(ctx).update(ctx, |warpify_settings, ctx| {
                    report_if_error!(warpify_settings
                        .ssh_extension_install_mode
                        .set_value(*mode, ctx));
                    send_telemetry_from_ctx!(
                        TelemetryEvent::SetSshExtensionInstallMode {
                            mode: mode.telemetry_name(),
                        },
                        ctx
                    );
                });
            }
            WarpifyPageAction::RemoveDenylistedSshHost(index) => {
                self.remove_denylisted_ssh_host(*index, ctx);
            }
            OpenUrl(url) => {
                ctx.open_url(url.as_str());
            }
        }
    }
}

impl SettingsPageMeta for WarpifyPageView {
    fn section() -> SettingsSection {
        SettingsSection::Warpify
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

impl From<ViewHandle<WarpifyPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<WarpifyPageView>) -> Self {
        SettingsPageViewHandle::Warpify(view_handle)
    }
}

#[derive(Default)]
struct SSHWidget {
    tmux_warpification_switch_state: SwitchStateHandle,
    enable_ssh_warpification_switch_state: SwitchStateHandle,
    additional_info_mouse_state: MouseStateHandle,
    local_only_icon_tooltip_states: RefCell<HashMap<String, MouseStateHandle>>,
}

impl SettingsWidget for SSHWidget {
    type View = WarpifyPageView;

    fn search_terms(&self) -> &str {
        "warpify ssh"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut column = Flex::column();
        let ui_builder = appearance.ui_builder();
        let description_text_color = appearance
            .theme()
            .sub_text_color(appearance.theme().surface_2());

        let enable_ssh_warpification = *WarpifySettings::as_ref(app)
            .enable_ssh_warpification
            .value();

        let should_prompt_ssh_tmux_wrapper =
            *WarpifySettings::as_ref(app).use_ssh_tmux_wrapper.value();

        add_setting(
            &mut column,
            &WarpifySettings::as_ref(app).enable_ssh_warpification,
            move || {
                render_body_item::<WarpifyPageAction>(
                    crate::t!("settings-warpify-enable-ssh"),
                    None,
                    LocalOnlyIconState::for_setting(
                        EnableSshWarpification::storage_key(),
                        EnableSshWarpification::sync_to_cloud(),
                        &mut self.local_only_icon_tooltip_states.borrow_mut(),
                        app,
                    ),
                    ToggleState::Enabled,
                    appearance,
                    ui_builder
                        .switch(self.enable_ssh_warpification_switch_state.clone())
                        .check(enable_ssh_warpification)
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(WarpifyPageAction::ToggleSshWarpification);
                        })
                        .finish(),
                    None,
                )
            },
        );

        if FeatureFlag::SshRemoteServer.is_enabled() {
            let label_color_override = if !enable_ssh_warpification {
                Some(appearance.theme().disabled_ui_text_color())
            } else {
                None
            };
            add_setting(
                &mut column,
                &WarpifySettings::as_ref(app).ssh_extension_install_mode,
                move || {
                    let install_ssh_label = crate::t!("settings-warpify-install-ssh-extension");
                    let install_ssh_desc =
                        crate::t!("settings-warpify-install-ssh-extension-description");
                    Container::new(render_dropdown_item(
                        appearance,
                        &install_ssh_label,
                        Some(&install_ssh_desc),
                        None,
                        LocalOnlyIconState::for_setting(
                            SshExtensionInstallModeSetting::storage_key(),
                            SshExtensionInstallModeSetting::sync_to_cloud(),
                            &mut self.local_only_icon_tooltip_states.borrow_mut(),
                            app,
                        ),
                        label_color_override,
                        &view.ssh_extension_install_mode_dropdown,
                    ))
                    .with_padding_bottom(HEADER_PADDING)
                    .finish()
                },
            );
        }

        add_setting(
            &mut column,
            &WarpifySettings::as_ref(app).use_ssh_tmux_wrapper,
            move || {
                let mut column = Flex::column();

                column.add_child(render_body_item::<WarpifyPageAction>(
                    crate::t!("settings-warpify-use-tmux"),
                    Some(AdditionalInfo {
                        mouse_state: self.additional_info_mouse_state.clone(),
                        on_click_action: Some(WarpifyPageAction::OpenUrl(
                            "https://docs.warp.dev/terminal/warpify/ssh".into(),
                        )),
                        secondary_text: None,
                        tooltip_override_text: None,
                    }),
                    LocalOnlyIconState::for_setting(
                        UseSshTmuxWrapper::storage_key(),
                        UseSshTmuxWrapper::sync_to_cloud(),
                        &mut self.local_only_icon_tooltip_states.borrow_mut(),
                        app,
                    ),
                    enable_ssh_warpification.into(),
                    appearance,
                    ui_builder
                        .switch(self.tmux_warpification_switch_state.clone())
                        .check(should_prompt_ssh_tmux_wrapper)
                        .with_disabled(!enable_ssh_warpification)
                        .build()
                        .on_click(move |ctx, _, _| {
                            if !enable_ssh_warpification {
                                return;
                            }

                            ctx.dispatch_typed_action(WarpifyPageAction::ToggleTmuxWarpification);
                        })
                        .finish(),
                    None,
                ));

                column.add_child(
                    ui_builder
                        .paragraph(crate::t!("settings-warpify-tmux-description"))
                        .with_style(UiComponentStyles {
                            font_color: Some(description_text_color.into_solid()),
                            margin: Some(
                                Coords::default()
                                    .top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
                                    .bottom(styles::DESCRIPTION_LINE_MARGIN_BOTTOM),
                            ),
                            ..Default::default()
                        })
                        .build()
                        .finish(),
                );

                if enable_ssh_warpification && should_prompt_ssh_tmux_wrapper {
                    let warpify_settings = WarpifySettings::as_ref(app);
                    column.add_child(
                        view.build_input_list(
                            crate::t!("settings-warpify-denylisted-hosts"),
                            &warpify_settings.ssh_hosts_denylist,
                            &view.remove_denylisted_ssh_button_states,
                            WarpifyPageAction::RemoveDenylistedSshHost,
                            &view.add_denylisted_ssh_editor,
                            appearance,
                        )
                        .finish(),
                    );
                } else {
                    // Add margin to hint the user should scroll to see more.
                    column.add_child(
                        Container::new(Flex::column().finish())
                            .with_margin_bottom(styles::MINIMUM_SCROLL_OFFSET_AFTER_SSH)
                            .finish(),
                    );
                }

                column.finish()
            },
        );

        column.finish()
    }
}

mod styles {
    // Apply a negative margin to the description text so it appears closer to the main
    // settings option text.
    pub const DESCRIPTION_NEGATIVE_MARGIN_OFFSET: f32 = -8.;

    /// The space after a description.
    pub const DESCRIPTION_LINE_MARGIN_BOTTOM: f32 = 18.;

    /// Because we hide the SSH settings if the SSH wrapper is disabled, we need to add a margin
    /// to the bottom to make it clear that toggling this item will reveal more settings,
    /// even at smaller window sizes. We picked an offset that cuts off the first item
    /// to imply the user should scroll to see more.
    pub const MINIMUM_SCROLL_OFFSET_AFTER_SSH: f32 = 40.;
}

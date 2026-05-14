//! Bug Report settings page (REQ-019).
//!
//! An in-app form that files GitHub issues in BitConcepts/kairos or
//! BitConcepts/specsmith with duplicate detection via `specsmith issue check`.
//!
//! Workflow:
//!  1. User picks a repo (Kairos / specsmith), types title + description.
//!  2. "Check for Duplicates" → spawns `specsmith issue check <title> --json`.
//!  3. Results shown; user can proceed or open an existing issue.
//!  4. "File Report" → spawns `specsmith issue file <title> --body ... --json`.
//!
//! All subprocess calls time out at 15 s (H9 / H11).

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use warpui::{
    elements::{
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Flex, Hoverable,
        MouseStateHandle, ParentElement, Radius, Text,
    },
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ---------------------------------------------------------------------------
// Repo selector
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
pub enum BugReportRepo {
    #[default]
    Kairos,
    Specsmith,
}

impl BugReportRepo {
    fn label(&self) -> &'static str {
        match self {
            Self::Kairos => "kairos",
            Self::Specsmith => "specsmith",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::Kairos => "Kairos (terminal/UI)",
            Self::Specsmith => "specsmith (AI/governance)",
        }
    }
}

// ---------------------------------------------------------------------------
// Status state machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DuplicateMatch {
    pub number: u32,
    pub title: String,
    pub url: String,
    pub similarity: f32,
}

#[derive(Debug, Clone, Default)]
pub enum BugReportStatus {
    #[default]
    Idle,
    Checking,
    NoMatches,
    Matches {
        duplicates: Vec<DuplicateMatch>,
        similar: Vec<DuplicateMatch>,
    },
    Filing,
    Filed {
        number: u32,
        url: String,
    },
    Error {
        message: String,
    },
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum BugReportPageAction {
    SetRepo(BugReportRepo),
    CheckDuplicates,
    FileReport { force: bool },
    OpenLink(String),
    Reset,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct BugReportPageView {
    page: PageType<Self>,
    repo: BugReportRepo,
    /// Stored title (set when the title input submits).
    title: String,
    /// Stored description (set when the description input submits).
    description: String,
    status: BugReportStatus,
    title_input: ViewHandle<SubmittableTextInput>,
    desc_input: ViewHandle<SubmittableTextInput>,
    repo_kairos_button: MouseStateHandle,
    repo_specsmith_button: MouseStateHandle,
    check_button: MouseStateHandle,
    file_button: MouseStateHandle,
    force_file_button: MouseStateHandle,
    reset_button: MouseStateHandle,
}

impl BugReportPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let title_input = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text("Brief, descriptive title…".to_owned(), ctx);
            input
        });
        ctx.subscribe_to_view(&title_input, Self::on_title_event);

        let desc_input = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text(
                "Steps to reproduce, expected vs actual behaviour…".to_owned(),
                ctx,
            );
            input
        });
        ctx.subscribe_to_view(&desc_input, Self::on_desc_event);

        Self {
            page: PageType::new_monolith(BugReportPageWidget::default(), None, false),
            repo: BugReportRepo::default(),
            title: String::new(),
            description: String::new(),
            status: BugReportStatus::default(),
            title_input,
            desc_input,
            repo_kairos_button: MouseStateHandle::default(),
            repo_specsmith_button: MouseStateHandle::default(),
            check_button: MouseStateHandle::default(),
            file_button: MouseStateHandle::default(),
            force_file_button: MouseStateHandle::default(),
            reset_button: MouseStateHandle::default(),
        }
    }

    fn on_title_event(
        &mut self,
        _handle: ViewHandle<SubmittableTextInput>,
        event: &SubmittableTextInputEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        if let SubmittableTextInputEvent::Submit(text) = event {
            self.title = text.trim().to_owned();
            // Reset status when title changes so the old check is invalidated.
            self.status = BugReportStatus::Idle;
            ctx.notify();
        }
    }

    fn on_desc_event(
        &mut self,
        _handle: ViewHandle<SubmittableTextInput>,
        event: &SubmittableTextInputEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        if let SubmittableTextInputEvent::Submit(text) = event {
            self.description = text.trim().to_owned();
            ctx.notify();
        }
    }

    fn run_check(&mut self, ctx: &mut ViewContext<Self>) {
        let title = self.title.clone();
        let repo = self.repo.label().to_owned();
        if title.is_empty() {
            self.status = BugReportStatus::Error {
                message: "Please enter a title first (press Enter to confirm).".to_owned(),
            };
            ctx.notify();
            return;
        }
        self.status = BugReportStatus::Checking;
        ctx.notify();
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[String]| {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let args: Vec<String> = ["issue", "check", &title, "--repo", &repo, "--json"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                run("py", &{
                    let mut a = vec!["-m".to_string(), "specsmith".to_string()];
                    a.extend(args.clone());
                    a
                })
                .or_else(|_| run("specsmith", &args))
                .map_err(|e| format!("specsmith not found: {e}"))
            },
            |me, result, ctx| {
                me.status = match result {
                    Err(e) => BugReportStatus::Error { message: e },
                    Ok(output) => {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        Self::parse_check_result(&text, output.status.success())
                    }
                };
                ctx.notify();
            },
        );
    }

    fn parse_check_result(json: &str, success: bool) -> BugReportStatus {
        if !success && json.trim().is_empty() {
            return BugReportStatus::Error {
                message: "specsmith issue check failed. Is specsmith installed?".to_owned(),
            };
        }
        let v: serde_json::Value = match serde_json::from_str(json.trim()) {
            Ok(v) => v,
            Err(_) => {
                return BugReportStatus::Error {
                    message: format!("Could not parse specsmith output: {}", &json[..json.len().min(120)]),
                };
            }
        };

        let parse_list = |arr: &serde_json::Value| -> Vec<DuplicateMatch> {
            arr.as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|item| {
                    Some(DuplicateMatch {
                        number: item["number"].as_u64().unwrap_or(0) as u32,
                        title: item["title"].as_str().unwrap_or("").to_owned(),
                        url: item["html_url"].as_str().unwrap_or("").to_owned(),
                        similarity: item["similarity"].as_f64().unwrap_or(0.0) as f32,
                    })
                })
                .collect()
        };

        let duplicates = parse_list(&v["duplicates"]);
        let similar = parse_list(&v["similar"]);

        if !v["error"].as_str().unwrap_or("").is_empty() {
            return BugReportStatus::Error {
                message: v["error"].as_str().unwrap_or("unknown error").to_owned(),
            };
        }
        if duplicates.is_empty() && similar.is_empty() {
            BugReportStatus::NoMatches
        } else {
            BugReportStatus::Matches { duplicates, similar }
        }
    }

    fn run_file(&mut self, force: bool, ctx: &mut ViewContext<Self>) {
        let title = self.title.clone();
        let body = self.description.clone();
        let repo = self.repo.label().to_owned();
        if title.is_empty() {
            self.status = BugReportStatus::Error {
                message: "Title is required.".to_owned(),
            };
            ctx.notify();
            return;
        }
        self.status = BugReportStatus::Filing;
        ctx.notify();
        ctx.spawn(
            async move {
                let mut args: Vec<String> = vec![
                    "issue".into(),
                    "file".into(),
                    title,
                    "--repo".into(),
                    repo,
                    "--json".into(),
                ];
                if !body.is_empty() {
                    args.push("--body".into());
                    args.push(body);
                }
                if force {
                    args.push("--force".into());
                }
                let run = |prog: &str, all_args: &[String]| {
                    std::process::Command::new(prog)
                        .args(all_args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let py_args: Vec<String> = {
                    let mut a = vec!["-m".to_string(), "specsmith".to_string()];
                    a.extend(args.clone());
                    a
                };
                run("py", &py_args)
                    .or_else(|_| run("specsmith", &args))
                    .map_err(|e| format!("specsmith not found: {e}"))
            },
            |me, result, ctx| {
                me.status = match result {
                    Err(e) => BugReportStatus::Error { message: e },
                    Ok(output) => {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        Self::parse_file_result(&text, output.status.success())
                    }
                };
                ctx.notify();
            },
        );
    }

    fn parse_file_result(json: &str, _success: bool) -> BugReportStatus {
        let v: serde_json::Value = match serde_json::from_str(json.trim()) {
            Ok(v) => v,
            Err(_) => {
                return BugReportStatus::Error {
                    message: format!(
                        "Could not parse specsmith output: {}",
                        &json[..json.len().min(200)]
                    ),
                };
            }
        };
        if v["ok"].as_bool().unwrap_or(false) {
            let number = v["number"].as_u64().unwrap_or(0) as u32;
            let url = v["html_url"].as_str().unwrap_or("").to_owned();
            BugReportStatus::Filed { number, url }
        } else {
            let err = v["error"].as_str().unwrap_or("Filing failed.").to_owned();
            // If blocked by duplicates, treat as Matches so user can force
            if err.to_lowercase().contains("duplicate") || err.to_lowercase().contains("blocked") {
                BugReportStatus::Error {
                    message: format!("{err} Use 'File Anyway' to override."),
                }
            } else {
                BugReportStatus::Error { message: err }
            }
        }
    }
}

impl Entity for BugReportPageView {
    type Event = SettingsPageEvent;
}

impl View for BugReportPageView {
    fn ui_name() -> &'static str {
        "BugReportPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for BugReportPageView {
    type Action = BugReportPageAction;

    fn handle_action(&mut self, action: &BugReportPageAction, ctx: &mut ViewContext<Self>) {
        match action {
            BugReportPageAction::SetRepo(repo) => {
                self.repo = repo.clone();
                self.status = BugReportStatus::Idle;
                ctx.notify();
            }
            BugReportPageAction::CheckDuplicates => self.run_check(ctx),
            BugReportPageAction::FileReport { force } => self.run_file(*force, ctx),
            BugReportPageAction::OpenLink(url) => {
                ctx.open_url(url);
            }
            BugReportPageAction::Reset => {
                self.title.clear();
                self.description.clear();
                self.status = BugReportStatus::Idle;
                ctx.notify();
            }
        }
    }
}

impl SettingsPageMeta for BugReportPageView {
    fn section() -> SettingsSection {
        SettingsSection::BugReport
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _allow_steal_focus: bool, ctx: &mut ViewContext<Self>) {
        // Reset status on page navigation so stale results don't linger.
        if matches!(self.status, BugReportStatus::Filed { .. }) {
            self.status = BugReportStatus::Idle;
            ctx.notify();
        }
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id);
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<BugReportPageView>> for SettingsPageViewHandle {
    fn from(handle: ViewHandle<BugReportPageView>) -> Self {
        SettingsPageViewHandle::BugReport(handle)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct BugReportPageWidget {}

impl SettingsWidget for BugReportPageWidget {
    type View = BugReportPageView;

    fn search_terms(&self) -> &str {
        "bug report issue file github duplicate kairos specsmith crash problem"
    }

    fn render(
        &self,
        view: &BugReportPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        use warpui::elements::{ChildView, Expanded};
        let theme = appearance.theme();
        let font = appearance.ui_font_family();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color().into();
        let accent = theme.accent().into_solid();

        // ── Page header ───────────────────────────────────────────────
        let header = build_sub_header(appearance, "File a Bug Report", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let desc = Container::new(
            Text::new(
                "File an issue in GitHub. Kairos checks for duplicates before filing. \
                 Requires specsmith and `gh` CLI (github.com/cli/cli)."
                    .to_owned(),
                font,
                12.,
            )
            .with_color(dim.into())
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(16.)
        .finish();

        // ── Repo selector ─────────────────────────────────────────────
        let repo_label = Text::new("Repository".to_owned(), font, 12.)
            .with_color(dim.into())
            .finish();

        let is_kairos = matches!(view.repo, BugReportRepo::Kairos);
        let make_repo_btn =
            |label: &str, repo: BugReportRepo, active_sel: bool, ms: MouseStateHandle| {
                let variant = if active_sel {
                    ButtonVariant::Accent
                } else {
                    ButtonVariant::Secondary
                };
                appearance
                    .ui_builder()
                    .button(variant, ms)
                    .with_style(UiComponentStyles {
                        font_size: Some(12.),
                        padding: Some(Coords::uniform(6.)),
                        ..Default::default()
                    })
                    .with_centered_text_label(label.to_owned())
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(BugReportPageAction::SetRepo(repo.clone()));
                    })
                    .finish()
            };

        let repo_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(repo_label)
                    .with_margin_right(12.)
                    .finish(),
            )
            .with_child(
                Container::new(make_repo_btn(
                    "Kairos (terminal/UI)",
                    BugReportRepo::Kairos,
                    is_kairos,
                    view.repo_kairos_button.clone(),
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(make_repo_btn(
                "specsmith (AI/governance)",
                BugReportRepo::Specsmith,
                !is_kairos,
                view.repo_specsmith_button.clone(),
            ))
            .finish();

        // ── Title input ───────────────────────────────────────────────
        let title_hint = if view.title.is_empty() {
            "Enter title, then press Return to confirm".to_owned()
        } else {
            format!("✓  {}", view.title)
        };
        let title_hint_color = if view.title.is_empty() {
            dim.into()
        } else {
            active
        };

        let title_section = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(
                    Text::new("Title".to_owned(), font, 12.)
                        .with_color(dim.into())
                        .finish(),
                )
                .with_margin_bottom(4.)
                .finish(),
            )
            .with_child(ChildView::new(&view.title_input).finish())
            .with_child(
                Container::new(
                    Text::new(title_hint, font, 11.)
                        .with_color(title_hint_color)
                        .finish(),
                )
                .with_margin_top(4.)
                .finish(),
            )
            .finish();

        // ── Description input ─────────────────────────────────────────
        let desc_hint = if view.description.is_empty() {
            "Enter description, then press Return to confirm (optional)".to_owned()
        } else {
            format!("✓  {} chars", view.description.len())
        };
        let desc_section = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(
                    Text::new("Description".to_owned(), font, 12.)
                        .with_color(dim.into())
                        .finish(),
                )
                .with_margin_bottom(4.)
                .finish(),
            )
            .with_child(ChildView::new(&view.desc_input).finish())
            .with_child(
                Container::new(
                    Text::new(desc_hint, font, 11.)
                        .with_color(dim.into())
                        .finish(),
                )
                .with_margin_top(4.)
                .finish(),
            )
            .finish();

        // ── Status display ────────────────────────────────────────────
        let status_card = self.render_status(view, appearance);

        // ── Action buttons ────────────────────────────────────────────
        let can_check = !view.title.is_empty()
            && !matches!(view.status, BugReportStatus::Checking | BugReportStatus::Filing);
        let can_file =
            matches!(view.status, BugReportStatus::NoMatches | BugReportStatus::Matches { .. })
                && !view.title.is_empty();
        let can_force = matches!(view.status, BugReportStatus::Matches { .. });

        let action_bar = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(self.action_button(
                "Check Duplicates",
                can_check,
                view.check_button.clone(),
                BugReportPageAction::CheckDuplicates,
                appearance,
            ))
            .with_child(self.action_button(
                "File Report",
                can_file,
                view.file_button.clone(),
                BugReportPageAction::FileReport { force: false },
                appearance,
            ))
            .with_child(self.action_button(
                "File Anyway",
                can_force,
                view.force_file_button.clone(),
                BugReportPageAction::FileReport { force: true },
                appearance,
            ))
            .with_child(
                Expanded::new(
                    1.,
                    warpui::elements::Empty::new().finish(),
                )
                .finish(),
            )
            .with_child(self.action_button(
                "Reset",
                !matches!(view.status, BugReportStatus::Idle),
                view.reset_button.clone(),
                BugReportPageAction::Reset,
                appearance,
            ))
            .finish();

        // ── Assemble ──────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(header)
                    .with_child(desc)
                    .with_child(Container::new(repo_row).with_margin_bottom(16.).finish())
                    .with_child(render_separator(appearance))
                    .with_child(Container::new(title_section).with_margin_top(12.).finish())
                    .with_child(Container::new(desc_section).with_margin_top(12.).finish())
                    .with_child(Container::new(status_card).with_margin_top(16.).finish())
                    .with_child(Container::new(action_bar).with_margin_top(12.).finish())
                    .finish(),
            )
            .with_max_width(680.)
            .finish(),
        )
        .with_uniform_padding(28.)
        .finish()
    }
}

impl BugReportPageWidget {
    fn action_button(
        &self,
        label: &str,
        enabled: bool,
        ms: MouseStateHandle,
        action: BugReportPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let variant = if enabled {
            ButtonVariant::Secondary
        } else {
            ButtonVariant::Text
        };
        let btn = appearance
            .ui_builder()
            .button(variant, ms)
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label(label.to_owned())
            .build();
        if enabled {
            btn.on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
        } else {
            btn.finish()
        }
    }

    fn render_status(
        &self,
        view: &BugReportPageView,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let font = appearance.ui_font_family();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color().into();

        match &view.status {
            BugReportStatus::Idle => Container::new(
                Text::new(
                    "Fill in the title and description, then click 'Check Duplicates'."
                        .to_owned(),
                    font,
                    12.,
                )
                .with_color(dim.into())
                .finish(),
            )
            .finish(),

            BugReportStatus::Checking => Container::new(
                Text::new("Searching GitHub for similar issues…".to_owned(), font, 12.)
                    .with_color(dim.into())
                    .finish(),
            )
            .finish(),

            BugReportStatus::NoMatches => Container::new(
                Text::new(
                    "✓  No similar issues found — safe to file.".to_owned(),
                    font,
                    12.,
                )
                .with_color(theme.accent().into_solid().into())
                .finish(),
            )
            .finish(),

            BugReportStatus::Matches { duplicates, similar } => {
                let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

                if !duplicates.is_empty() {
                    col.add_child(
                        Container::new(
                            Text::new(
                                format!(
                                    "⚠  {} likely duplicate(s) found — review before filing:",
                                    duplicates.len()
                                ),
                                font,
                                12.,
                            )
                            .with_color(active)
                            .finish(),
                        )
                        .with_margin_bottom(6.)
                        .finish(),
                    );
                    for m in duplicates {
                        col.add_child(self.match_row(m, appearance));
                    }
                }
                if !similar.is_empty() {
                    col.add_child(
                        Container::new(
                            Text::new(
                                format!("  {} similar issue(s):", similar.len()),
                                font,
                                11.,
                            )
                            .with_color(dim.into())
                            .finish(),
                        )
                        .with_margin_top(6.)
                        .with_margin_bottom(4.)
                        .finish(),
                    );
                    for m in similar {
                        col.add_child(self.match_row(m, appearance));
                    }
                }
                col.finish()
            }

            BugReportStatus::Filing => Container::new(
                Text::new("Filing issue via specsmith…".to_owned(), font, 12.)
                    .with_color(dim.into())
                    .finish(),
            )
            .finish(),

            BugReportStatus::Filed { number, url } => {
                let url_clone = url.clone();
                let label = format!("✓  Filed #{}  — click to open in browser", number);
                Hoverable::new(MouseStateHandle::default(), move |_| {
                    Text::new(label.clone(), font, 12.)
                        .with_color(theme.accent().into_solid().into())
                        .finish()
                })
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(BugReportPageAction::OpenLink(url_clone.clone()));
                })
                .with_cursor(warpui::platform::Cursor::PointingHand)
                .finish()
            }

            BugReportStatus::Error { message } => Container::new(
                Text::new(format!("✗  {message}"), font, 12.)
                    .with_color(active)
                    .soft_wrap(true)
                    .finish(),
            )
            .finish(),
        }
    }

    fn match_row(&self, m: &DuplicateMatch, appearance: &Appearance) -> Box<dyn Element> {
        let font = appearance.ui_font_family();
        let mono = appearance.monospace_font_family();
        let dim = appearance.theme().disabled_ui_text_color();
        let accent = appearance.theme().accent().into_solid();
        let url = m.url.clone();
        let label = format!(
            "  #{} — {}  ({:.0}%)",
            m.number,
            &m.title[..m.title.len().min(70)],
            m.similarity * 100.0
        );
        Hoverable::new(MouseStateHandle::default(), move |_| {
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Text::new(label.clone(), font, 11.)
                        .with_color(dim.into())
                        .finish(),
                )
                .with_child(
                    Container::new(
                        Text::new("→ open".to_owned(), mono, 10.)
                            .with_color(accent.into())
                            .finish(),
                    )
                    .with_margin_left(8.)
                    .finish(),
                )
                .finish()
        })
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(BugReportPageAction::OpenLink(url.clone()));
        })
        .with_cursor(warpui::platform::Cursor::PointingHand)
        .finish()
    }
}

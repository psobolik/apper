mod desktop_entry;
mod theme;

use crate::desktop_entry::DesktopEntry;
use configparser::ini::Ini;
use cursive::Cursive;
use cursive::CursiveRunnable;
use cursive::event::{Event, EventResult};
use cursive::reexports::log::LevelFilter;
use cursive::style::{ColorStyle, Style};
use cursive::theme::BaseColor::Yellow;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::utils::span::SpannedString;
use cursive::view::ViewWrapper;
use cursive::views::{Dialog, LinearLayout, NamedView};
use cursive::views::{OnEventView, SelectView, TextView};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

const DEFAULT_STATUS_TEXT: &str = " ";
const STATUS_TEXT_VIEW: &str = "statusTextView";
const SELECT_VIEW: &str = "selectView";

#[tokio::main]
async fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();
    cursive::logger::set_internal_filter_level(LevelFilter::Info);

    let mut siv = cursive::default();
    siv.set_theme(theme::theme());
    add_global_callbacks(&mut siv);
    siv.add_layer(main_view().await);

    // This rigmarole makes sure the comment for the initially selected item is displayed.
    // It seems a lot more complicated than it should be.
    if let Some(desktop_entry) = siv
        .call_on_name(
            SELECT_VIEW,
            |s: &mut NamedView<SelectView<DesktopEntry>>| s.with_view(|s| s.selection().unwrap()),
        )
        .unwrap()
    {
        show_comment(&mut *siv, desktop_entry.as_ref())
    }

    siv.run();
}

fn add_global_callbacks(siv: &mut CursiveRunnable) {
    siv.add_global_callback('~', Cursive::toggle_debug_console);
    siv.add_global_callback(Event::CtrlChar('q'), |s| s.quit());
    siv.add_global_callback(Event::CtrlChar('c'), |s| s.quit());
}
async fn main_view() -> impl View {
    let status_text_view = TextView::new(DEFAULT_STATUS_TEXT).with_name(STATUS_TEXT_VIEW);

    LinearLayout::vertical()
        .child(select_view().await)
        .child(status_text_view)
}

async fn select_view() -> impl View {
    /// Format the desktop entry for use in the select list
    pub fn format(desktop_entry: &DesktopEntry) -> String {
        format!(
            "{}",
            desktop_entry
                .name
                .clone()
                .unwrap_or_else(|| "<No name>".to_string())
        )
    }
    // Create the (named) SelectView
    let mut select_view = SelectView::new()
        .autojump()
        .on_submit(spawn_command)
        .on_select(show_comment)
        .with_name(SELECT_VIEW);

    // Add the desktop entries to the SelectView
    match desktop_entries().await {
        Ok(desktop_entries) => {
            select_view.with_view_mut(|s| {
                s.add_all(desktop_entries.into_iter().map(|de| (format(&de), de)))
            });
        }
        Err(e) => eprint!("{}", e),
    }

    // Wrap the SelectView in an OnEventView with some key event handlers
    OnEventView::new(select_view)
        .on_pre_event_inner(Event::CtrlChar('k'), |s, _| {
            s.with_view_mut(|s| {
                let cb = s.select_up(1);
                EventResult::Consumed(Some(cb))
            })
        })
        .on_pre_event_inner(Event::CtrlChar('j'), |s, _| {
            s.with_view_mut(|s| {
                let cb = s.select_down(1);
                EventResult::Consumed(Some(cb))
            })
        })
        .scrollable()
        .full_screen()
}

async fn desktop_entries() -> Result<Vec<DesktopEntry>, Box<dyn Error>> {
    let user_path = format!(
        "{}/.local/share/applications",
        std::env::var("HOME").unwrap_or(String::from("/root"))
    );
    let mut entries = desktop_entries_in_path(user_path).await?;
    entries.append(&mut desktop_entries_in_path("/usr/share/applications").await?);
    // Sort by name
    entries.sort_unstable_by(|rhs, lhs| rhs.name.cmp(&lhs.name));
    // Filter out commands that run in the terminal
    let filtered = entries.into_iter().filter(|de| !de.terminal).collect();
    Ok(filtered)
}

async fn desktop_entries_in_path(
    path: impl AsRef<Path>,
) -> Result<Vec<DesktopEntry>, Box<dyn Error>> {
    let desktop_files = desktop_files(path).await?;
    let mut entries = vec![];
    for desktop_file in desktop_files {
        let mut config = Ini::new();
        let _map = config.load(desktop_file);
        let no_display = match config.get("desktop entry", "NoDisplay") {
            Some(value) => value.eq("true"),
            None => false,
        };
        let terminal = match config.get("desktop entry", "Terminal") {
            Some(value) => value.eq("true"),
            None => false,
        };
        if !no_display {
            entries.push(DesktopEntry {
                name: config.get("Desktop Entry", "Name"),
                generic_name: config.get("Desktop Entry", "GenericName"),
                comment: config.get("Desktop Entry", "Comment"),
                exec: config.get("Desktop Entry", "Exec"),
                try_exec: config.get("Desktop Entry", "TryExec"),
                categories: None,
                mime_type: config.get("Desktop Entry", "MimeType"),
                terminal,
                no_display,
            })
        }
    }
    Ok(entries)
}

async fn desktop_files(path: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let mut path_bufs: Vec<PathBuf> = vec![];
    let mut entries = fs::read_dir(&path).await?;
    while let Some(dir_entry) = entries.next_entry().await? {
        if dir_entry.path().is_file() {
            if let Some(extension) = dir_entry.path().extension() {
                if extension == "desktop" {
                    path_bufs.push(dir_entry.path());
                }
            }
        }
    }
    Ok(path_bufs.iter().map(|path| path.clone()).collect())
}

/// When a desktop entry is "submitted", we spawn the executable and quit the program
fn spawn_command(siv: &mut Cursive, desktop_entry: &DesktopEntry) {
    match desktop_entry.command() {
        Some(command) => {
            siv.quit();
            Command::new(command).spawn().expect("Failed to spawn");
        }
        None => {
            let dialog = Dialog::around(TextView::new("No Command?")).button("Ok", |s| {
                s.pop_layer();
            });
            siv.add_layer(dialog);
        }
    }
}

/// When a desktop entry is selected, we display its comment at the bottom of the app window
fn show_comment(siv: &mut Cursive, selected_entry: &DesktopEntry) {
    let status = if let Some(comment) = selected_entry.comment.clone() {
        StyledString::single_span(
            comment,
            Style {
                effects: Default::default(),
                color: ColorStyle::front(Yellow),
            },
        )
    } else {
        SpannedString::from(DEFAULT_STATUS_TEXT)
    };
    siv.call_on_name(STATUS_TEXT_VIEW, |view: &mut TextView| {
        view.set_content(status)
    });
}

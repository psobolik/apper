mod desktop_entry;
mod theme;
mod user_data;

use crate::desktop_entry::DesktopEntry;
use crate::user_data::UserData;

use cursive::Cursive;
use cursive::CursiveRunnable;
use cursive::event::{Event, EventResult, Key};
use cursive::reexports::log::{LevelFilter, debug};
use cursive::style::{ColorStyle, Style};
use cursive::theme::BaseColor::Yellow;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::utils::span::SpannedString;
use cursive::views::{Dialog, LinearLayout, OnEventView, SelectView, TextView};
use fork::{Fork, daemon};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

const CATEGORIES_VIEW_NAME: &str = "categories";
const PROGRAMS_VIEW_NAME: &str = "programs";
const DEFAULT_STATUS_TEXT: &str = " ";
const STATUS_TEXT_VIEW: &str = "statusTextView";
const ALL_CATEGORIES: &str = "\0";
const NO_CATEGORY: &str = "";

#[tokio::main]
async fn main() {
    // Initialize the cursive logger when debugging
    if cfg!(debug_assertions) {
        cursive::logger::init();
        cursive::logger::set_internal_filter_level(LevelFilter::Info);
        debug!("Program started");
    }
    let mut siv = cursive::default();
    siv.set_user_data(UserData {
        desktop_entries: desktop_entries().await,
    });
    siv.set_theme(theme::theme());
    add_global_callbacks(&mut siv);
    siv.add_layer(app_view());
    init_categories_list(&mut siv);
    update_programs_list(&mut siv, &ALL_CATEGORIES.to_string());

    siv.run();
}

fn add_global_callbacks(siv: &mut CursiveRunnable) {
    if cfg!(debug_assertions) {
        siv.add_global_callback('~', Cursive::toggle_debug_console);
    }
    siv.add_global_callback(Event::CtrlChar('q'), |s| s.quit());
    siv.add_global_callback(Event::CtrlChar('c'), |s| s.quit());
}
fn app_view() -> impl View {
    let status_text_view = TextView::new(DEFAULT_STATUS_TEXT).with_name(STATUS_TEXT_VIEW);

    LinearLayout::vertical()
        .child(main_view())
        .child(status_text_view)
}
fn main_view() -> impl View {
    fn categories_list_view() -> impl View {
        let wrapped_view = SelectView::new()
            .autojump()
            .on_select(update_programs_list)
            .on_submit(|siv, _: &String| {
                siv.focus_name(PROGRAMS_VIEW_NAME).unwrap();
            })
            .with_name(CATEGORIES_VIEW_NAME)
            .scrollable()
            .wrap_with(OnEventView::new)
            .on_pre_event_inner(Event::CtrlChar('n'), |view, _event| {
                view.on_event(Event::Key(Key::Down));
                Some(EventResult::Consumed(None))
            })
            .on_pre_event_inner(Event::CtrlChar('p'), |view, _event| {
                view.on_event(Event::Key(Key::Up));
                Some(EventResult::Consumed(None))
            });
        Dialog::around(wrapped_view)
            .title("Categories")
            .full_screen()
    }
    fn programs_list_view() -> impl View {
        let wrapped_view = SelectView::new()
            .autojump()
            .on_select(show_comment)
            .on_submit(spawn_command)
            .with_name(PROGRAMS_VIEW_NAME)
            .scrollable()
            .wrap_with(OnEventView::new)
            .on_pre_event_inner(Event::CtrlChar('n'), |view, _event| {
                view.on_event(Event::Key(Key::Down));
                Some(EventResult::Consumed(None))
            })
            .on_pre_event_inner(Event::CtrlChar('p'), |view, _event| {
                view.on_event(Event::Key(Key::Up));
                Some(EventResult::Consumed(None))
            });
        Dialog::around(wrapped_view).title("Programs").full_screen()
    }

    LinearLayout::horizontal()
        .child(categories_list_view())
        .child(programs_list_view())
}

/// Get a list of desktop files in the canonic user and system directories
/// and return them sorted by name in a vector of DesktopEntry structs.
async fn desktop_entries() -> Vec<DesktopEntry> {
    let user_path = format!(
        "{}/.local/share/applications",
        std::env::var("HOME").unwrap_or(String::from("/root"))
    );
    let mut entries: Vec<DesktopEntry> = vec![];
    if let Ok(mut user) = desktop_entries_in_path(user_path).await {
        entries.append(&mut user);
    };
    if let Ok(mut system) = desktop_entries_in_path("/usr/share/applications").await {
        entries.append(&mut system);
    };
    // Sort by name
    entries.sort_unstable_by(|rhs, lhs| rhs.name.cmp(&lhs.name));
    // Filter out commands that run in the terminal
    entries.into_iter().filter(|de| !de.terminal).collect()
}

/// Get a list of desktop files in a given directory and return them in a vector of DesktopEntry structs.
async fn desktop_entries_in_path(
    path: impl AsRef<Path>,
) -> Result<Vec<DesktopEntry>, Box<dyn Error>> {
    let desktop_files = desktop_files(path).await?;
    let mut entries = vec![];
    for desktop_file in desktop_files {
        if let Ok(desktop_entry) = DesktopEntry::read(desktop_file)
            && !desktop_entry.no_display
        {
            entries.push(desktop_entry);
        }
    }
    Ok(entries)
}
/// Get a list of files with the "desktop" extension in a given folder
async fn desktop_files(path: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let mut path_bufs: Vec<PathBuf> = vec![];
    let mut entries = fs::read_dir(&path).await?;
    while let Some(dir_entry) = entries.next_entry().await? {
        if dir_entry.path().is_file()
            && let Some(extension) = dir_entry.path().extension()
            && extension == "desktop"
        {
            path_bufs.push(dir_entry.path());
        }
    }
    Ok(path_bufs.to_vec())
}
/// Called when a desktop entry is "submitted", spawns the executable in a new process group and quit the program
fn spawn_command(siv: &mut Cursive, desktop_entry: &DesktopEntry) {
    match desktop_entry.command() {
        Some(command) => {
            siv.quit();
            if let Ok(Fork::Child) = daemon(true, true) {
                Command::new(command).spawn().expect("Failed to spawn");
            }
        }
        None => {
            let dialog = Dialog::around(TextView::new("No Command?")).button("Ok", |s| {
                s.pop_layer();
            });
            siv.add_layer(dialog);
        }
    }
}

/// Called when a desktop entry is selected, displays its comment at the bottom of the app window
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
/// Get a sorted list of the distinct categories in the list of directory entries.
/// Include a blank category to represent "no category" and a fake category to represent "any category".
/// Return the list as a vector of tuples for use in the SelectList
fn categories(entries: &Vec<DesktopEntry>) -> Vec<(String, String)> {
    let mut unique_categories: Vec<(String, String)> = vec![];

    for entry in entries {
        if let Some(entry_categories) = &entry.categories {
            for entry_category in entry_categories {
                let item = (entry_category.clone(), entry_category.clone());
                if !unique_categories.contains(&item) {
                    unique_categories.push(item.clone())
                }
            }
        }
    }
    unique_categories.sort();

    let mut categories: Vec<(String, String)> = vec![
        (
            String::from("<all categories>").clone(),
            String::from(ALL_CATEGORIES),
        ),
        (
            String::from("<no category>").clone(),
            String::from(NO_CATEGORY),
        ),
    ];
    categories.append(&mut unique_categories);
    categories
}
/// Display the categories in a SelectView
fn init_categories_list(siv: &mut Cursive) {
    let mut categories_view = siv.find_name::<SelectView>(CATEGORIES_VIEW_NAME).unwrap();
    siv.with_user_data(|user_data: &mut UserData| {
        categories_view.clear();
        categories_view.add_all(categories(&user_data.desktop_entries));
    });
}
/// Get a sorted list of the desktop entries from the full list that are in the given category.
/// Return the list as a vector of tuples for use in the SelectList
fn programs(entries: &Vec<DesktopEntry>, category: &str) -> Vec<(String, DesktopEntry)> {
    let mut programs = vec![];
    for entry in entries {
        if let Some(name) = entry.name.clone() {
            let item = (name, entry.clone());
            if category == ALL_CATEGORIES {
                programs.push(item);
            } else {
                match entry.categories.clone() {
                    Some(entry_categories) => {
                        if entry_categories.contains(&category.to_string()) {
                            programs.push(item);
                        }
                    }
                    None => {
                        if category == NO_CATEGORY {
                            programs.push(item);
                        }
                    }
                }
            }
        }
    }
    programs.sort();
    programs
}
/// Called when a category is selected, displays the programs in that category
fn update_programs_list(siv: &mut Cursive, category: &String) {
    if let Some(mut programs_view) = siv.find_name::<SelectView<DesktopEntry>>(PROGRAMS_VIEW_NAME) {
        siv.with_user_data(|user_data: &mut UserData| {
            programs_view.clear();
            programs_view.add_all(programs(&user_data.desktop_entries, category));
        });
        if let Some(selected_entry) = programs_view.selection() {
            show_comment(siv, &selected_entry);
        }
    }
}

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct DesktopEntry {
    pub name: Option<String>,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub try_exec: Option<String>,
    pub categories: Option<Vec<String>>,
    pub mime_type: Option<Vec<String>>,
    pub no_display: bool,
    pub terminal: bool,
}

impl DesktopEntry {
    // Tip: A desktop file is not an INI file! An INI file parser will interpret the semicolons that separate
    // mutliple values as comment delimiters.
    pub fn read(path: impl AsRef<Path>,) -> io::Result<Self> {
        fn split_values(values: &str) -> Option<Vec<String>> {
            let v = values.split(";")
                .filter_map(|value| if !value.is_empty() { Some(value.to_string()) } else { None })
                .collect();
            // If there are no categories in the file, return None 
            if values.is_empty() {
                None
            } else {
                Some(v)
            }
        }
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);

        let header_pat = regex::Regex::new(r"^\[.*\]$").unwrap();
        let mut in_header = false;
        let mut result = Self::default();
        for line in reader.lines() {
            match line {
                Ok(raw_line) => {
                    let line = raw_line.trim();
                    if line.starts_with("#") { continue; } // Skip comments
                    else if header_pat.is_match(line) {
                        in_header = line == "[Desktop Entry]";
                    } else if in_header { // Only read lines in a [Desktop Entry] section
                        if let Some((key, value)) = line.split_once("=") {
                            match key.trim().to_lowercase().as_str() {
                                "name" => result.name = Some(value.trim().to_string()),
                                "genericname" => result.generic_name = Some(value.trim().to_string()),
                                "comment" => result.comment = Some(value.trim().to_string()),
                                "exec" => result.exec = Some(value.trim().to_string()),
                                "tryexec" => result.try_exec = Some(value.trim().to_string()),
                                "categories" => result.categories = split_values(value.trim()),
                                "mimetype" => result.mime_type = split_values(value.trim()),
                                "nodisplay" => result.no_display = value.trim().eq_ignore_ascii_case("true"),
                                "terminal" => result.terminal = value.trim().eq_ignore_ascii_case("true"),
                                _ => { /* Ignore anything we don't support */}
                            }
                        }
                    }
                }
                // Abandon the loop if there's an error reading a line from the file
                Err(e) => return Err(e)
            }
        }
        Ok(result)
    }
    /// Get the exec value as a string, without quotes or parameters.
    // The %f, %F, %u and %U parameters are irrelevant to this app, but we ignore %i, %c and %k as
    // well, more from laziness than anything else.
    // (https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html)
    pub fn command(&self) -> Option<String> {
        if let Some(exec) = self.exec.clone() {
            let regex = regex::Regex::new("^(?<command>.+?)(?: %[fFuUick])*$").unwrap();
            let captures = regex.captures(&exec).unwrap();
            captures
                .name("command")
                .map(|command| command.as_str().trim_matches('"').to_string())
        } else {
            None
        }
    }
}


impl PartialOrd for DesktopEntry {
    fn partial_cmp(&self, other: &DesktopEntry) -> Option<std::cmp::Ordering> {
       Some(self.cmp(other))
    }
}

impl Ord for DesktopEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

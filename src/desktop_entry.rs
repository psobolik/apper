#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: Option<String>,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub try_exec: Option<String>,
    pub categories: Option<Vec<String>>,
    pub mime_type: Option<String>,
    pub no_display: bool,
    pub terminal: bool,
}

impl DesktopEntry {
    /// Get the exec value as a string, without quotes or parameters.
    // The %f, %F, %u and %U parameters are irrelevant to this app, but we ignore %i, %c and %k as
    // well, more from laziness than anything else.
    // (https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html)
    pub fn command(&self) -> Option<String> {
        if let Some(exec) = self.exec.clone() {
            let regex = regex::Regex::new("^(?<command>.+?)(?: %[fFuUick])*$").unwrap();
            let captures = regex.captures(&*exec).unwrap();
            match captures.name("command") {
                Some(command) => Some(command.as_str().trim_matches('"').to_string()),
                None => None,
            }
        } else {
            None
        }
    }
}
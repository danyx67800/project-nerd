//! UI-only state that is not persisted in the evening file.

use std::time::Instant;

use nerd_core::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    Timeline,
    Titles,
    Live,
    Settings,
    Gallery,
}

impl Page {
    /// Pages shown in the bottom page bar. The component gallery is hidden
    /// behind the `--gallery` launch flag.
    pub const ALL: [Page; 5] = [
        Page::Library,
        Page::Timeline,
        Page::Titles,
        Page::Live,
        Page::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Library => crate::strings::PAGE_LIBRARY,
            Page::Timeline => crate::strings::PAGE_TIMELINE,
            Page::Titles => crate::strings::PAGE_TITLES,
            Page::Live => crate::strings::PAGE_LIVE,
            Page::Settings => crate::strings::SETTINGS,
            Page::Gallery => "Galleria componenti",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "libreria" | "library" => Some(Self::Library),
            "scaletta" | "timeline" => Some(Self::Timeline),
            "titoli" | "titles" => Some(Self::Titles),
            "live" => Some(Self::Live),
            "impostazioni" | "settings" => Some(Self::Settings),
            "galleria" | "gallery" => Some(Self::Gallery),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Page::Library => 0,
            Page::Timeline => 1,
            Page::Titles => 2,
            Page::Live => 3,
            Page::Settings => 4,
            Page::Gallery => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub born: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>, kind: ToastKind) -> Self {
        Self {
            message: message.into(),
            kind,
            born: Instant::now(),
        }
    }
}

/// Wrapper so the overlay can be stored separately from transient toasts.
#[derive(Debug, Clone)]
pub struct Snackbar {
    pub toast: Toast,
}

#[derive(Debug, Default)]
pub struct UndoStack {
    undo: Vec<Project>,
    redo: Vec<Project>,
    last_push: Option<Instant>,
}

impl UndoStack {
    pub fn reset(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_push = None;
    }

    /// Saves a snapshot. Rapid edits are coalesced into one undo step.
    pub fn checkpoint(&mut self, project: &Project) {
        let now = Instant::now();
        if self
            .last_push
            .is_some_and(|last| now.duration_since(last) < std::time::Duration::from_millis(650))
        {
            return;
        }
        self.last_push = Some(now);
        self.undo.push(project.clone());
        if self.undo.len() > 80 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: &Project) -> Option<Project> {
        let previous = self.undo.pop()?;
        self.redo.push(current.clone());
        self.last_push = None;
        Some(previous)
    }

    pub fn redo(&mut self, current: &Project) -> Option<Project> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        self.last_push = None;
        Some(next)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Checklist {
    pub audio_output: bool,
    pub projector: bool,
    pub volumes: bool,
}

impl Checklist {
    pub fn complete(self) -> bool {
        self.audio_output && self.projector && self.volumes
    }
}

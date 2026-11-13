use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    SideBySide,
    Single,
}

#[derive(Default)]
pub struct Comparison {
    pub paths: Vec<PathBuf>,
    pub selected: usize,
    pub mode: ViewMode,
}

impl Comparison {
    pub fn add(&mut self, paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
        let mut added = Vec::new();
        for path in paths {
            let path = path.canonicalize().unwrap_or(path);
            if !self.paths.contains(&path) {
                self.paths.push(path.clone());
                added.push(path);
            }
        }
        added
    }

    pub fn next(&mut self) {
        if !self.paths.is_empty() {
            self.selected = (self.selected + 1) % self.paths.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.paths.is_empty() {
            self.selected = (self.selected + self.paths.len() - 1) % self.paths.len();
        }
    }

    pub fn toggle(&mut self) {
        let mode = match self.mode {
            ViewMode::SideBySide => ViewMode::Single,
            ViewMode::Single => ViewMode::SideBySide,
        };
        self.set_mode(mode);
    }

    pub fn set_mode(&mut self, mode: ViewMode) {
        if self.mode != mode && mode == ViewMode::Single {
            self.selected = 0;
        }
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_single_view_always_starts_at_first_image() {
        let mut model = Comparison::default();
        model.add(["a.png".into(), "b.png".into(), "c.png".into()]);
        model.set_mode(ViewMode::Single);
        model.previous();
        assert_eq!(model.selected, 2);
        model.next();
        assert_eq!(model.selected, 0);
        model.next();
        model.set_mode(ViewMode::Single);
        assert_eq!(
            model.selected, 1,
            "reselecting the current mode keeps navigation"
        );
        model.set_mode(ViewMode::SideBySide);
        model.set_mode(ViewMode::Single);
        assert_eq!(model.selected, 0, "toolbar transition starts at image one");
        model.next();
        model.toggle();
        model.toggle();
        assert_eq!(model.mode, ViewMode::Single);
        assert_eq!(model.selected, 0, "Tab transition starts at image one");
    }

    #[test]
    fn empty_and_single_navigation_are_safe() {
        let mut model = Comparison::default();
        model.previous();
        model.next();
        assert_eq!(model.selected, 0);
        model.add(["only.png".into()]);
        model.next();
        model.previous();
        assert_eq!(model.selected, 0);
    }

    #[test]
    fn add_preserves_order_and_deduplicates() {
        let mut model = Comparison::default();
        let added = model.add(["b.png".into(), "a.png".into(), "b.png".into()]);
        assert_eq!(added, vec![PathBuf::from("b.png"), PathBuf::from("a.png")]);
        assert!(model.add(["a.png".into()]).is_empty());
    }
}

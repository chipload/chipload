// SPDX-License-Identifier: GPL-3.0-or-later
//! The window's panels, docked round the canvas (docs/specs/dockable-panels.md, ADR 0028):
//! the Browser, Properties, the shape library (Shapes) and the Report, arranged by the user
//! with egui_dock. The canvas stays in place: no tab bar, never closed, moved or tabbed.

use chipload_job::layout::{
    LAYOUT_FILE, LayoutNode, Panel as LayoutPanel, WindowLayout, load_layout, save_layout,
};
use egui_dock::{
    DockArea, DockState, Node, NodeIndex, Split, SurfaceIndex, TabIndex, TabPath, TabViewer, Tree,
};

use super::*;

/// The least width of the Properties forms: they lay out down to it (narrower labels and
/// fields, rows that wrap) and scroll sideways below it. A docked panel can be dragged
/// narrower; the narrow-window test checks the forms at this width.
#[cfg(test)]
pub(crate) const PROPERTIES_MIN_WIDTH: f32 = 250.0;

/// The default layout's shares, giving today's widths in a 1200 × 800 window: the Browser
/// 340, the right panel 440, the Report folded along the bottom. A split's share is its
/// first side's: the left one, or the upper one.
const BROWSER_SHARE: f32 = 0.28;
const CANVAS_BESIDE_RIGHT: f32 = 0.49;
const ABOVE_REPORT: f32 = 0.82;

/// What the dock shows: the canvas, or one of the panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pane {
    Canvas,
    Browser,
    Properties,
    Shapes,
    Report,
}

impl Pane {
    /// The panels a user can arrange, close and open again.
    pub const PANELS: [Pane; 4] = [Pane::Browser, Pane::Properties, Pane::Shapes, Pane::Report];

    /// Its title on its tab and in View ▸ Panels.
    pub fn title(self, lang: &impl Lang) -> String {
        match self {
            Pane::Canvas => String::new(),
            Pane::Browser => t!(lang, "browser-caption"),
            Pane::Properties => t!(lang, "app-properties"),
            Pane::Shapes => t!(lang, "shape-library-tab"),
            Pane::Report => t!(lang, "report-heading"),
        }
    }
}

impl From<LayoutPanel> for Pane {
    fn from(panel: LayoutPanel) -> Self {
        match panel {
            LayoutPanel::Browser => Pane::Browser,
            LayoutPanel::Properties => Pane::Properties,
            LayoutPanel::Shapes => Pane::Shapes,
            LayoutPanel::Report => Pane::Report,
        }
    }
}

impl Pane {
    /// The panel as the layout file names it; None for the canvas.
    fn panel(self) -> Option<LayoutPanel> {
        match self {
            Pane::Canvas => None,
            Pane::Browser => Some(LayoutPanel::Browser),
            Pane::Properties => Some(LayoutPanel::Properties),
            Pane::Shapes => Some(LayoutPanel::Shapes),
            Pane::Report => Some(LayoutPanel::Report),
        }
    }
}

/// A change to the layout asked for while the dock is being drawn, made once it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Show(Pane),
    Front(Pane),
    Fold(Pane),
}

/// The panels' arrangement.
pub struct Panels {
    dock: DockState<Pane>,
    /// While P hides the panels: the layout to bring back.
    hidden: Option<DockState<Pane>>,
    /// The dock is out being drawn: changes wait in `pending`, and `open` and `shown`
    /// answer for it.
    drawing: bool,
    pending: Vec<Change>,
    open: BTreeSet<Pane>,
    /// While the dock is drawn: the panels shown when its drawing began.
    shown: BTreeSet<Pane>,
    /// The layout as last read from or written to `layout.json`; written again only when
    /// it differs.
    pub(super) saved: Option<WindowLayout>,
}

impl Default for Panels {
    fn default() -> Self {
        Panels::from_layout(&WindowLayout::default())
    }
}

/// The tabs a part of the layout starts with: its first group's, or the canvas.
fn first_tabs(node: &LayoutNode) -> Vec<Pane> {
    match node {
        LayoutNode::Canvas => vec![Pane::Canvas],
        LayoutNode::Tabs { panels, .. } => panels.iter().map(|p| Pane::from(*p)).collect(),
        LayoutNode::SideBySide { left, .. } => first_tabs(left),
        LayoutNode::Stacked { top, .. } => first_tabs(top),
    }
}

/// Build `node` at `at`, a leaf already holding `first_tabs(node)`: a split keeps that leaf
/// as its left or upper side and gets its other side new. Folded groups are noted in
/// `folded`, to fold once the whole tree is built.
fn build(tree: &mut Tree<Pane>, at: NodeIndex, node: &LayoutNode, folded: &mut Vec<NodeIndex>) {
    let share = |s: f64| (s as f32).clamp(0.05, 0.95);
    match node {
        LayoutNode::Canvas => {}
        LayoutNode::Tabs {
            panels,
            front,
            folded: fold,
        } => {
            if let Some(leaf) = tree[at].get_leaf_mut() {
                leaf.active = TabIndex((*front).min(panels.len().saturating_sub(1)));
            }
            if *fold {
                folded.push(at);
            }
        }
        LayoutNode::SideBySide {
            share: s,
            left,
            right,
        } => {
            let second = Node::leaf_with(first_tabs(right));
            let [l, r] = tree.split(at, Split::Right, share(*s), second);
            build(tree, l, left, folded);
            build(tree, r, right, folded);
        }
        LayoutNode::Stacked {
            share: s,
            top,
            bottom,
        } => {
            let second = Node::leaf_with(first_tabs(bottom));
            let [t, b] = tree.split(at, Split::Below, share(*s), second);
            build(tree, t, top, folded);
            build(tree, b, bottom, folded);
        }
    }
}

/// The part of the dock at `at` as a layout; None for an empty part.
fn describe(tree: &Tree<Pane>, at: NodeIndex) -> Option<LayoutNode> {
    // Shares are kept to 1/1000: what a drag sets, without the last bits of a float.
    let share = |f: f32| (f64::from(f) * 1000.0).round() / 1000.0;
    let both = |left: Option<LayoutNode>, right: Option<LayoutNode>| match (left, right) {
        (Some(l), Some(r)) => Some((l, r)),
        _ => None,
    };
    match &tree[at] {
        Node::Empty => None,
        Node::Leaf(leaf) => {
            if leaf.tabs.contains(&Pane::Canvas) {
                return Some(LayoutNode::Canvas);
            }
            let panels: Vec<LayoutPanel> = leaf.tabs.iter().filter_map(|t| t.panel()).collect();
            (!panels.is_empty()).then_some(LayoutNode::Tabs {
                front: leaf.active.0.min(panels.len() - 1),
                panels,
                folded: leaf.collapsed,
            })
        }
        Node::Horizontal(split) => {
            let (left, right) = (describe(tree, at.left()), describe(tree, at.right()));
            match both(left.clone(), right.clone()) {
                Some((l, r)) => Some(LayoutNode::SideBySide {
                    share: share(split.fraction),
                    left: Box::new(l),
                    right: Box::new(r),
                }),
                None => left.or(right),
            }
        }
        Node::Vertical(split) => {
            let (top, bottom) = (describe(tree, at.left()), describe(tree, at.right()));
            match both(top.clone(), bottom.clone()) {
                Some((t, b)) => Some(LayoutNode::Stacked {
                    share: share(split.fraction),
                    top: Box::new(t),
                    bottom: Box::new(b),
                }),
                None => top.or(bottom),
            }
        }
    }
}

impl Panels {
    /// The panels as `layout` arranges them.
    pub fn from_layout(layout: &WindowLayout) -> Panels {
        let mut dock = DockState::new(first_tabs(&layout.root));
        let mut folded = Vec::new();
        let tree = dock.main_surface_mut();
        build(tree, NodeIndex::root(), &layout.root, &mut folded);
        for node in folded {
            set_folded(tree, node, true);
        }
        let mut panels = Panels {
            dock,
            hidden: None,
            drawing: false,
            pending: Vec::new(),
            open: BTreeSet::new(),
            shown: BTreeSet::new(),
            saved: None,
        };
        panels.settle();
        panels
    }

    /// The layout to keep: the panels as arranged, or as they come back after P.
    pub fn layout(&self) -> WindowLayout {
        let dock = self.hidden.as_ref().unwrap_or(&self.dock);
        let root = describe(dock.main_surface(), NodeIndex::root()).unwrap_or(LayoutNode::Canvas);
        WindowLayout {
            root,
            ..WindowLayout::default()
        }
    }

    /// Whether `pane` is open (in front or behind another tab, folded or not).
    pub fn is_open(&self, pane: Pane) -> bool {
        self.open.contains(&pane)
    }

    /// Whether `pane` is the tab in front of its panel and the panel isn't folded.
    pub fn is_shown(&self, pane: Pane) -> bool {
        if self.drawing {
            return self.shown.contains(&pane);
        }
        let Some((node, tab)) = self.dock.find_main_surface_tab(&pane) else {
            return false;
        };
        let tree = self.dock.main_surface();
        tree[node]
            .get_leaf()
            .is_some_and(|leaf| leaf.active == tab && !leaf.collapsed)
    }

    /// Bring `pane` to the front, unfolded, opening it where it goes by default when it was
    /// closed, and the panels back when P hid them.
    pub fn show(&mut self, pane: Pane) {
        if self.drawing {
            self.pending.push(Change::Show(pane));
            return;
        }
        if let Some(dock) = self.hidden.take() {
            self.dock = dock;
        }
        if self.dock.find_main_surface_tab(&pane).is_none() {
            self.open_at_default(pane);
        }
        if let Some((node, tab)) = self.dock.find_main_surface_tab(&pane) {
            let tree = self.dock.main_surface_mut();
            // The tab was just found there: setting it active can't fail.
            let _ = tree.set_active_tab(node, tab);
            set_folded(tree, node, false);
        }
        self.settle();
    }

    /// Bring `pane` to the front of its panel if it is open, leaving it folded or not.
    pub fn bring_to_front(&mut self, pane: Pane) {
        if self.drawing {
            self.pending.push(Change::Front(pane));
            return;
        }
        if let Some((node, tab)) = self.dock.find_main_surface_tab(&pane) {
            // The tab was just found there: setting it active can't fail.
            let _ = self.dock.main_surface_mut().set_active_tab(node, tab);
        }
    }

    /// Fold `pane`'s panel to its tab bar.
    pub fn fold(&mut self, pane: Pane) {
        if self.drawing {
            self.pending.push(Change::Fold(pane));
            return;
        }
        if let Some((node, _)) = self.dock.find_main_surface_tab(&pane) {
            set_folded(self.dock.main_surface_mut(), node, true);
        }
    }

    /// Close `pane`; View ▸ Panels opens it again.
    pub fn close(&mut self, pane: Pane) {
        if pane == Pane::Canvas || self.drawing {
            return;
        }
        if let Some((node, tab)) = self.dock.find_main_surface_tab(&pane) {
            self.dock
                .remove_tab(TabPath::new(SurfaceIndex::main(), node, tab));
        }
        self.settle();
    }

    /// P: hide every panel for the largest canvas, or bring them back as they were.
    pub fn toggle_hidden(&mut self) {
        match self.hidden.take() {
            Some(dock) => self.dock = dock,
            None => {
                let all = std::mem::replace(&mut self.dock, DockState::new(vec![Pane::Canvas]));
                self.hidden = Some(all);
            }
        }
        self.settle();
    }

    /// Whether P has hidden the panels.
    pub fn hidden(&self) -> bool {
        self.hidden.is_some()
    }

    /// View ▸ Reset layout.
    pub fn reset(&mut self) {
        let saved = self.saved.take();
        *self = Panels::default();
        self.saved = saved;
    }

    /// A panel dropped into the canvas's middle, as a drag there leaves it in egui_dock:
    /// what `settle` then makes of it.
    #[cfg(test)]
    pub fn drop_on_canvas(&mut self, pane: Pane) {
        self.close(pane);
        if let Some((canvas, _)) = self.dock.find_main_surface_tab(&Pane::Canvas)
            && let Some(leaf) = self.dock.main_surface_mut()[canvas].get_leaf_mut()
        {
            leaf.tabs.push(pane);
        }
        self.settle();
    }

    /// Whether `a` and `b` are tabs of one panel.
    #[cfg(test)]
    pub fn together(&self, a: Pane, b: Pane) -> bool {
        let node = |p: Pane| self.dock.find_main_surface_tab(&p).map(|(n, _)| n);
        node(a).is_some() && node(a) == node(b)
    }

    /// `pane`, closed, put back where the default layout has it: the Browser left of the
    /// canvas, Properties and Shapes together right of it, the Report below everything.
    fn open_at_default(&mut self, pane: Pane) {
        let partner = match pane {
            Pane::Properties => Some(Pane::Shapes),
            Pane::Shapes => Some(Pane::Properties),
            _ => None,
        };
        if let Some((node, _)) = partner.and_then(|p| self.dock.find_main_surface_tab(&p)) {
            let tree = self.dock.main_surface_mut();
            if let Some(leaf) = tree[node].get_leaf_mut() {
                leaf.tabs.push(pane);
            }
            return;
        }
        let Some((canvas, _)) = self.dock.find_main_surface_tab(&Pane::Canvas) else {
            return;
        };
        let tree = self.dock.main_surface_mut();
        match pane {
            Pane::Browser => {
                tree.split_left(canvas, BROWSER_SHARE, vec![pane]);
            }
            Pane::Report => {
                tree.split_below(NodeIndex::root(), ABOVE_REPORT, vec![pane]);
            }
            _ => {
                tree.split_right(canvas, CANVAS_BESIDE_RIGHT, vec![pane]);
            }
        }
    }

    /// After any change: the canvas alone in its leaf with no tab bar (a panel dropped into
    /// its middle docks beside it instead), and which panels are open.
    fn settle(&mut self) {
        if let Some((canvas, _)) = self.dock.find_main_surface_tab(&Pane::Canvas) {
            let tree = self.dock.main_surface_mut();
            let others: Vec<Pane> = tree[canvas]
                .get_leaf_mut()
                .map(|leaf| {
                    let others = leaf.tabs.iter().copied().filter(|t| *t != Pane::Canvas);
                    let others: Vec<Pane> = others.collect();
                    leaf.tabs.retain(|t| *t == Pane::Canvas);
                    leaf.active = TabIndex(0);
                    leaf.collapsed = false;
                    others
                })
                .unwrap_or_default();
            tree[canvas].set_tab_bar_hidden(true);
            if !others.is_empty() {
                tree.split_right(canvas, CANVAS_BESIDE_RIGHT, others);
            }
        }
        self.open = self
            .dock
            .main_surface()
            .iter()
            .filter_map(|node| node.tabs())
            .flatten()
            .copied()
            .collect();
    }
}

/// Fold or unfold leaf `node`, and keep its splits' counts of folded leaves right: a split
/// side by side is as folded as its more folded side, one above the other as both together,
/// and fully folded when both its sides are.
fn set_folded(tree: &mut Tree<Pane>, node: NodeIndex, folded: bool) {
    tree[node].set_collapsed(folded);
    let mut at = node.parent();
    while let Some(split) = at {
        let (left, right) = (&tree[split.left()], &tree[split.right()]);
        let (l, r) = (left.collapsed_leaf_count(), right.collapsed_leaf_count());
        let both = left.is_collapsed() && right.is_collapsed();
        let count = if tree[split].is_horizontal() {
            l.max(r)
        } else {
            l + r
        };
        tree[split].set_collapsed_leaf_count(count);
        tree[split].set_collapsed(both);
        at = split.parent();
    }
}

/// Draws each pane's contents for the dock.
struct Viewer<'a> {
    app: &'a mut ChiploadApp,
    ctx: egui::Context,
}

impl TabViewer for Viewer<'_> {
    type Tab = Pane;

    fn id(&mut self, tab: &mut Pane) -> egui::Id {
        egui::Id::new(("pane", *tab))
    }

    fn title(&mut self, tab: &mut Pane) -> egui::WidgetText {
        let title = tab.title(self.app);
        // A folded Report says what it holds (docs/specs/dockable-panels.md, As built).
        let problems = self.app.report.problems();
        if *tab == Pane::Report && problems > 0 {
            let color = report::Tone::Warning.color(&self.ctx.global_style().visuals);
            return RichText::new(format!("{title} ⚠ {problems}"))
                .color(color)
                .into();
        }
        title.into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Pane) {
        match tab {
            Pane::Canvas => self.app.canvas_pane(ui, &self.ctx),
            Pane::Browser => self.app.panel_contents(ui, PanelKind::Browser),
            Pane::Properties => self.app.panel_contents(ui, PanelKind::Properties),
            Pane::Shapes => self.app.panel_contents(ui, PanelKind::Shapes),
            Pane::Report => self.app.report_pane(ui),
        }
    }

    fn on_tab_button(&mut self, tab: &mut Pane, response: &egui::Response) {
        // Named for screen readers (and the tests), as egui's own tabs are.
        let title = tab.title(self.app);
        let shown = self.app.panels.is_shown(*tab);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, shown, &title)
        });
        // The Shapes tab is where selected shapes can be dropped to save them, in front or
        // not; an entry dragged over it brings it to the front.
        if *tab == Pane::Shapes {
            self.app.shape_library.next_drop_rects.push(response.rect);
            if !shown
                && response.contains_pointer()
                && egui::DragAndDrop::has_payload_of_type::<shape_library::DraggedEntry>(&self.ctx)
            {
                self.app.panels.show(Pane::Shapes);
            }
        }
    }

    fn is_closeable(&self, tab: &Pane) -> bool {
        *tab != Pane::Canvas
    }

    fn allowed_in_windows(&self, _tab: &mut Pane) -> bool {
        false
    }

    fn tab_style_override(
        &self,
        tab: &Pane,
        global: &egui_dock::TabStyle,
    ) -> Option<egui_dock::TabStyle> {
        // The canvas fills its pane edge to edge.
        (*tab == Pane::Canvas).then(|| {
            let mut style = global.clone();
            style.tab_body.inner_margin = egui::Margin::ZERO;
            style
        })
    }

    fn scroll_bars(&self, _tab: &Pane) -> [bool; 2] {
        // Each panel scrolls in its own way (`panel_contents`); the canvas doesn't.
        [false, false]
    }
}

/// A panel's contents, wherever the panel is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelKind {
    Browser,
    Properties,
    Shapes,
}

impl ChiploadApp {
    /// A panel's contents in `ui`, scrolling where they need to.
    pub(super) fn panel_contents(&mut self, ui: &mut egui::Ui, kind: PanelKind) {
        // The forms are laid out in the panel's width; a row that still doesn't fit
        // scrolls sideways instead of widening the panel.
        let width = ui.available_width();
        match kind {
            PanelKind::Browser => self.sidebar(ui),
            PanelKind::Properties => {
                ScrollArea::both()
                    .id_salt("properties-scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.set_max_width(width);
                        self.properties_panel(ui);
                    });
            }
            PanelKind::Shapes => {
                ScrollArea::vertical()
                    .id_salt("shape-library-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(width);
                        self.shapes_tab(ui);
                    });
            }
        }
    }

    /// The canvas's pane: the update question above it, the playback bar below it in
    /// Simulate, and the canvas or, with no project, the start screen.
    fn canvas_pane(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Asked once whether to check for new versions (docs/specs/update-notice.md §2).
        if self.update_question_shown() {
            Panel::top("update-question").show(ui, |ui| self.update_bar(ui));
        }
        if self.edit.is_some() && self.playback.is_some() && self.display.simulate {
            Panel::bottom("playback").show(ui, |ui| self.playback_bar(ui));
            self.sync_playback(ctx);
        }
        if self.edit.is_some() {
            self.canvas(ui);
        } else {
            self.start_screen(ui);
        }
    }

    /// The layout kept in the settings folder, or the default one. A layout that can't be
    /// read (a newer version's, or a broken file) gives the default, says so, and stays on
    /// disk until the user changes the layout (docs/specs/dockable-panels.md §5).
    pub(super) fn load_layout(&mut self) {
        let Some(dir) = self.user.settings_dir.as_deref() else {
            return;
        };
        let path = dir.join(LAYOUT_FILE);
        let layout = match load_layout(&path) {
            Ok(layout) => layout,
            Err(err) => {
                let reason = err.message().render(self.lang(), self.prefs.units.into());
                let text = t!(self, "layout-not-read", reason = reason);
                self.set_status(text, ERROR_STATUS_SECONDS);
                WindowLayout::default()
            }
        };
        self.panels = Panels::from_layout(&layout);
        self.panels.saved = Some(self.panels.layout());
    }

    /// Keep the layout when it changed, once no drag is going on.
    fn save_layout_if_changed(&mut self, ctx: &egui::Context) {
        let Some(dir) = self.user.settings_dir.clone() else {
            return;
        };
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        let layout = self.panels.layout();
        if self.panels.saved.as_ref() == Some(&layout) {
            return;
        }
        match save_layout(&layout, &dir.join(LAYOUT_FILE)) {
            Ok(()) => self.panels.saved = Some(layout),
            Err(err) => {
                let reason = err.message().render(self.lang(), self.prefs.units.into());
                let text = t!(self, "layout-not-saved", reason = reason);
                self.set_status(text, ERROR_STATUS_SECONDS);
                // Not tried again until the layout changes once more.
                self.panels.saved = Some(layout);
            }
        }
    }

    /// The dock: the canvas and the panels round it, as the user arranged them.
    pub(super) fn dock_area(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let panels = &mut self.panels;
        panels.shown = panels
            .open
            .iter()
            .copied()
            .filter(|p| panels.is_shown(*p))
            .collect();
        let mut dock = std::mem::replace(&mut panels.dock, DockState::new(vec![Pane::Canvas]));
        panels.drawing = true;
        let mut style = egui_dock::Style::from_egui(ui.style());
        // The canvas's leaf has no tab bar, and nothing to show or drag it by.
        style.buttons.show_tab_bar_size = 0.0;
        style.tab.tab_body.hidden_tab_bar_drag_height = Some(0.0);
        style.tab.tab_body.inner_margin = egui::Margin::same(4);
        // No frame round the canvas or the dock: the panels' tab bars and dividers part them.
        style.tab.tab_body.stroke = egui::Stroke::NONE;
        style.main_surface_border_stroke = egui::Stroke::NONE;
        DockArea::new(&mut dock)
            .id(egui::Id::new("dock"))
            .style(style)
            .show_close_buttons(true)
            .show_leaf_collapse_buttons(true)
            .show_leaf_close_all_buttons(false)
            .show_add_buttons(false)
            .tab_context_menus(false)
            .hidable_tab_bars(true)
            .show_inside(
                ui,
                &mut Viewer {
                    app: self,
                    ctx: ctx.clone(),
                },
            );
        self.panels.dock = dock;
        self.panels.drawing = false;
        self.shape_library.drop_rects = std::mem::take(&mut self.shape_library.next_drop_rects);
        for change in std::mem::take(&mut self.panels.pending) {
            match change {
                Change::Show(pane) => self.panels.show(pane),
                Change::Front(pane) => self.panels.bring_to_front(pane),
                Change::Fold(pane) => self.panels.fold(pane),
            }
        }
        self.panels.settle();
        self.save_layout_if_changed(&ctx);
    }
}

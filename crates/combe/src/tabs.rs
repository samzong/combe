use std::collections::HashMap;

use objc2::rc::Retained;
use objc2_app_kit::NSView;

use crate::split;
use crate::surface::SurfaceView;

pub(crate) struct Tab {
    pub id: u64,
    pub workspace: String,
    pub name: String,
    pub label: String,
    pub root: Retained<NSView>,
    pub focused: Option<Retained<SurfaceView>>,
    pub zoom: Option<split::Zoom>,
}

impl Tab {
    pub(crate) fn contains(&self, view: &SurfaceView) -> bool {
        split::surfaces(&self.root)
            .iter()
            .any(|leaf| std::ptr::eq(&**leaf, view))
    }

    pub(crate) fn focused_surface(&self) -> Option<Retained<SurfaceView>> {
        let leaves = split::surfaces(&self.root);
        self.zoom
            .as_ref()
            .map(|zoom| zoom.surface.clone())
            .or_else(|| {
                self.focused
                    .as_ref()
                    .filter(|view| leaves.iter().any(|leaf| std::ptr::eq(&**leaf, &***view)))
                    .cloned()
            })
            .or_else(|| leaves.into_iter().next())
    }
}

#[derive(Default)]
pub(crate) struct Tabs {
    items: Vec<Tab>,
    active: HashMap<String, u64>,
    current: Option<String>,
    next_id: u64,
}

impl Tabs {
    pub(crate) fn items(&self) -> &[Tab] {
        &self.items
    }

    pub(crate) fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub(crate) fn visible(&self) -> impl Iterator<Item = &Tab> {
        let current = self.current.as_deref();
        self.items
            .iter()
            .filter(move |tab| Some(tab.workspace.as_str()) == current)
    }

    pub(crate) fn active_id(&self) -> Option<u64> {
        self.active.get(self.current.as_deref()?).copied()
    }

    pub(crate) fn active(&self) -> Option<&Tab> {
        self.get(self.active_id()?)
    }

    pub(crate) fn active_mut(&mut self) -> Option<&mut Tab> {
        self.get_mut(self.active_id()?)
    }

    pub(crate) fn owner(&self, view: &SurfaceView) -> Option<&Tab> {
        self.items.iter().find(|tab| tab.contains(view))
    }

    pub(crate) fn get(&self, id: u64) -> Option<&Tab> {
        self.items.iter().find(|tab| tab.id == id)
    }

    pub(crate) fn get_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.items.iter_mut().find(|tab| tab.id == id)
    }

    pub(crate) fn siblings(&self, id: u64) -> usize {
        let Some(tab) = self.get(id) else { return 0 };
        self.items
            .iter()
            .filter(|other| other.workspace == tab.workspace)
            .count()
    }

    pub(crate) fn other(&self, workspace: &str) -> Option<u64> {
        let next = self.items.iter().find(|tab| tab.workspace != workspace)?;
        self.active
            .get(&next.workspace)
            .copied()
            .filter(|id| self.get(*id).is_some())
            .or(Some(next.id))
    }

    pub(crate) fn enter(&mut self, workspace: &str) -> Option<u64> {
        self.current = Some(workspace.to_owned());
        let remembered = self
            .active
            .get(workspace)
            .copied()
            .filter(|id| self.get(*id).is_some());
        let pending = self.items.iter().find(|tab| {
            tab.workspace == workspace
                && split::surfaces(&tab.root)
                    .iter()
                    .any(|view| view.needs_attention())
        });
        let id = pending.map(|tab| tab.id).or(remembered).or_else(|| {
            self.items
                .iter()
                .find(|tab| tab.workspace == workspace)
                .map(|tab| tab.id)
        })?;
        self.active.insert(workspace.to_owned(), id);
        Some(id)
    }

    pub(crate) fn push(&mut self, workspace: String, name: String, root: Retained<NSView>) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.active.insert(workspace.clone(), id);
        self.current = Some(workspace.clone());
        self.items.push(Tab {
            id,
            workspace,
            label: name.clone(),
            name,
            root,
            focused: None,
            zoom: None,
        });
        id
    }

    pub(crate) fn set_active(&mut self, id: u64) {
        let Some(tab) = self.get(id) else { return };
        let workspace = tab.workspace.clone();
        self.active.insert(workspace.clone(), id);
        self.current = Some(workspace);
    }

    pub(crate) fn remove(&mut self, id: u64) -> Option<Tab> {
        let index = self.items.iter().position(|tab| tab.id == id)?;
        let tab = self.items.remove(index);
        if self.active.get(&tab.workspace) != Some(&id) {
            return Some(tab);
        }
        let next = self
            .items
            .iter()
            .skip(index)
            .chain(self.items.iter().take(index).rev())
            .find(|other| other.workspace == tab.workspace)
            .map(|other| other.id);
        match next {
            Some(next) => {
                self.active.insert(tab.workspace.clone(), next);
            }
            None => {
                self.active.remove(&tab.workspace);
                if self.current.as_deref() == Some(tab.workspace.as_str()) {
                    self.current = None;
                }
            }
        }
        Some(tab)
    }

    pub(crate) fn step(&self, delta: isize) -> Option<u64> {
        let ids: Vec<u64> = self.visible().map(|tab| tab.id).collect();
        if ids.is_empty() {
            return None;
        }
        let current = self
            .active_id()
            .and_then(|id| ids.iter().position(|other| *other == id))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(ids.len() as isize) as usize;
        Some(ids[next])
    }
}

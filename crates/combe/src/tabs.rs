use std::collections::HashMap;

use objc2::rc::Retained;
use objc2_app_kit::NSView;

use crate::split;
use crate::surface::SurfaceView;

pub struct Tab {
    pub id: u64,
    pub workspace: String,
    pub name: String,
    pub label: String,
    pub root: Retained<NSView>,
    pub focused: Option<Retained<SurfaceView>>,
    pub zoom: Option<crate::split::Zoom>,
}

impl Tab {
    pub fn focused_surface(&self) -> Option<Retained<SurfaceView>> {
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
pub struct Tabs {
    items: Vec<Tab>,
    active: HashMap<String, u64>,
    current: Option<String>,
    next_id: u64,
}

impl Tabs {
    pub fn items(&self) -> &[Tab] {
        &self.items
    }

    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn visible(&self) -> impl Iterator<Item = &Tab> {
        let current = self.current.as_deref();
        self.items
            .iter()
            .filter(move |tab| Some(tab.workspace.as_str()) == current)
    }

    pub fn active_id(&self) -> Option<u64> {
        self.active.get(self.current.as_deref()?).copied()
    }

    pub fn active(&self) -> Option<&Tab> {
        self.get(self.active_id()?)
    }

    pub fn get(&self, id: u64) -> Option<&Tab> {
        self.items.iter().find(|tab| tab.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.items.iter_mut().find(|tab| tab.id == id)
    }

    pub fn siblings(&self, id: u64) -> usize {
        let Some(tab) = self.get(id) else { return 0 };
        self.items
            .iter()
            .filter(|other| other.workspace == tab.workspace)
            .count()
    }

    pub fn other(&self, workspace: &str) -> Option<u64> {
        let next = self.items.iter().find(|tab| tab.workspace != workspace)?;
        self.active
            .get(&next.workspace)
            .copied()
            .filter(|id| self.get(*id).is_some())
            .or(Some(next.id))
    }

    pub fn enter(&mut self, workspace: &str) -> Option<u64> {
        self.current = Some(workspace.to_owned());
        let remembered = self
            .active
            .get(workspace)
            .copied()
            .filter(|id| self.get(*id).is_some());
        let id = remembered.or_else(|| {
            self.items
                .iter()
                .find(|tab| tab.workspace == workspace)
                .map(|tab| tab.id)
        })?;
        self.active.insert(workspace.to_owned(), id);
        Some(id)
    }

    pub fn push(&mut self, workspace: String, name: String, root: Retained<NSView>) -> u64 {
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

    pub fn set_active(&mut self, id: u64) {
        let Some(tab) = self.get(id) else { return };
        let workspace = tab.workspace.clone();
        self.active.insert(workspace.clone(), id);
        self.current = Some(workspace);
    }

    pub fn remove(&mut self, id: u64) -> Option<Tab> {
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

    pub fn step(&self, delta: isize) -> Option<u64> {
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

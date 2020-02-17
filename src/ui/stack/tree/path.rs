use super::super::super::super::ui;
use super::super::super::prelude::*;
use super::columns::Column;
use crate::nix_query_tree::exec_nix_store::NixStoreRes;
use crate::nix_query_tree::{NixQueryEntry, NixQueryTree, Recurse};
use crate::tree;
use std::collections::VecDeque;

/// This is a `gtk::TreePath` for the underlying non-sorted data.  This is the data that
/// corresponds 1-to-1 to the actual `NixStoreRes` data.
pub struct GtkChildTreePath(gtk::TreePath);

impl std::fmt::Debug for GtkChildTreePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GtkChildTreePath({:?})", self.to_path())
    }
}

impl GtkChildTreePath {
    pub fn new(tree_path: gtk::TreePath) -> Self {
        GtkChildTreePath(tree_path)
    }

    fn get(&self) -> &gtk::TreePath {
        &self.0
    }

    pub fn into_parent(
        &self,
        tree_model_sort: gtk::TreeModelSort,
    ) -> GtkParentTreePath {
        let parent_tree_path = tree_model_sort
            .convert_child_path_to_path(self.get())
            .expect("child_tree_path should always be able to be converted to a parent tree_path");
        GtkParentTreePath::new(parent_tree_path)
    }

    pub fn from_path(path: &tree::Path) -> Self {
        let mut vec_indices: Vec<i32> =
            path.0.iter().map(|&u| u as i32).collect();
        vec_indices.insert(0, 0);
        let gtk_child_tree_path =
            gtk::TreePath::new_from_indicesv(&vec_indices);
        GtkChildTreePath::new(gtk_child_tree_path)
    }

    pub fn to_path(&self) -> tree::Path {
        tree::Path(
            self.get()
                .get_indices()
                .iter()
                .map(|i| *i as usize)
                // our tree::Path will only ever have one item at the root, so we can drop the first
                // item from the gtk::TreePath.
                .skip(1)
                .collect::<VecDeque<usize>>(),
        )
    }

    pub fn nix_query_tree_lookup(
        &self,
        nix_query_tree: &NixQueryTree,
    ) -> Option<NixQueryEntry> {
        nix_query_tree.lookup(&self.to_path()).cloned()
    }

    pub fn nix_store_res_lookup(
        &self,
        nix_store_res: &NixStoreRes,
    ) -> Option<NixQueryEntry> {
        self.nix_query_tree_lookup(&nix_store_res.tree)
    }
}

/// This is a `gtk::TreePath` for the sorted model actually shown to the user.
///
/// This is just a "view" of the non-sorted data.
pub struct GtkParentTreePath(gtk::TreePath);

impl std::fmt::Debug for GtkParentTreePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GtkParentTreePath(actual: {:?})",
            GtkChildTreePath::new(self.0.clone()).to_path()
        )
    }
}

impl GtkParentTreePath {
    pub fn new(tree_path: gtk::TreePath) -> Self {
        GtkParentTreePath(tree_path)
    }

    fn get(&self) -> &gtk::TreePath {
        &self.0
    }

    pub fn into_child(
        &self,
        tree_model_sort: gtk::TreeModelSort,
    ) -> GtkChildTreePath {
        let parent_tree_path = tree_model_sort
            .convert_path_to_child_path(self.get())
            .expect("child_tree_path should always be able to be converted to a child_tree_path");
        GtkChildTreePath::new(parent_tree_path)
    }

    pub fn from_path(
        tree_model_sort: gtk::TreeModelSort,
        path: &tree::Path,
    ) -> Self {
        GtkChildTreePath::from_path(path).into_parent(tree_model_sort)
    }

    pub fn to_path(&self, tree_model_sort: gtk::TreeModelSort) -> tree::Path {
        self.into_child(tree_model_sort).to_path()
    }

    pub fn nix_query_tree_lookup(
        &self,
        tree_model_sort: gtk::TreeModelSort,
        nix_query_tree: &NixQueryTree,
    ) -> Option<NixQueryEntry> {
        self.into_child(tree_model_sort)
            .nix_query_tree_lookup(nix_query_tree)
    }

    pub fn nix_store_res_lookup(
        &self,
        tree_model_sort: gtk::TreeModelSort,
        nix_store_res: &NixStoreRes,
    ) -> Option<NixQueryEntry> {
        self.into_child(tree_model_sort)
            .nix_store_res_lookup(nix_store_res)
    }
}

/// This is a `gtk::TreeIter` for the underlying non-sorted data.  This is the data that
/// corresponds 1-to-1 to the actual `NixStoreRes` data.
pub struct GtkChildTreeIter(gtk::TreeIter);

impl GtkChildTreeIter {
    pub fn new(tree_iter: gtk::TreeIter) -> Self {
        GtkChildTreeIter(tree_iter)
    }

    fn get(&self) -> &gtk::TreeIter {
        &self.0
    }

    pub fn nix_store_res_lookup(
        &self,
        tree_store: gtk::TreeStore,
        nix_store_res: &NixStoreRes,
    ) -> Option<NixQueryEntry> {
        let tree_path = GtkChildTreePath::new(tree_store.get_path(self.get())?);
        tree_path.nix_query_tree_lookup(&nix_store_res.tree)
    }
}

// TODO: Is this function correct?
pub fn goto(state: &ui::State, first_path: &tree::Path) {
    let tree_view = state.get_tree_view();

    let tree_model_sort = state.get_tree_model_sort();
    let child_tree_path = GtkChildTreePath::from_path(first_path);
    let parent_tree_path = child_tree_path.into_parent(tree_model_sort);

    let col = tree_view.get_column(Column::Item as i32);

    // Open recursively upward from this new path.
    tree_view.expand_to_path(&parent_tree_path.get());

    // Scroll to the newly opened path.
    tree_view.scroll_to_cell(
        Some(&parent_tree_path.get()),
        col.as_ref(),
        true,
        0.5,
        0.5,
    );

    let tree_selection: gtk::TreeSelection = tree_view.get_selection();
    // Select the newly opened path.
    tree_selection.select_path(&parent_tree_path.get());
}

fn event_button_to_parent_tree_path_column(
    state: &ui::State,
    event_button: gdk::EventButton,
) -> Option<(GtkParentTreePath, gtk::TreeViewColumn)> {
    let tree_view = state.get_tree_view();
    let (x, y) = event_button.get_position();
    if let Some((Some(tree_path), Some(tree_view_column), _, _)) =
        tree_view.get_path_at_pos(x as i32, y as i32)
    {
        Some((GtkParentTreePath::new(tree_path), tree_view_column))
    } else {
        None
    }
}

fn event_button_to_child_tree_path_column(
    state: &ui::State,
    event_button: gdk::EventButton,
) -> Option<(GtkChildTreePath, gtk::TreeViewColumn)> {
    let tree_model_sort = state.get_tree_model_sort();
    event_button_to_parent_tree_path_column(state, event_button).map(
        |(parent_tree_path, tree_view_column)| {
            (
                parent_tree_path.into_child(tree_model_sort),
                tree_view_column,
            )
        },
    )
}

fn event_button_to_child_tree_path(
    state: &ui::State,
    event_button: gdk::EventButton,
) -> Option<GtkChildTreePath> {
    event_button_to_child_tree_path_column(state, event_button)
        .map(|tuple| tuple.0)
}

fn event_button_to_parent_tree_path(
    state: &ui::State,
    event_button: gdk::EventButton,
) -> Option<GtkParentTreePath> {
    event_button_to_parent_tree_path_column(state, event_button)
        .map(|tuple| tuple.0)
}

fn is_for_recurse_column_child(
    state: &ui::State,
    tree_view_column: gtk::TreeViewColumn,
    child_tree_path: GtkChildTreePath,
    nix_store_res: &NixStoreRes,
) -> Option<NixQueryEntry> {
    let tree_view = state.get_tree_view();
    let option_column =
        Column::from_gtk(tree_view.clone(), tree_view_column.clone());
    let option_nix_query_entry_is_recurse = child_tree_path
        .nix_store_res_lookup(nix_store_res)
        .filter(|nix_query_entry| nix_query_entry.1 == Recurse::Yes);

    match (option_column, option_nix_query_entry_is_recurse) {
        (Some(Column::Recurse), Some(nix_query_entry)) => Some(nix_query_entry),
        _ => None,
    }
}

pub fn is_for_recurse_column_parent(
    state: &ui::State,
    tree_view_column: gtk::TreeViewColumn,
    parent_tree_path: GtkParentTreePath,
    nix_store_res: &NixStoreRes,
) -> Option<NixQueryEntry> {
    let tree_model_sort = state.get_tree_model_sort();
    let child_tree_path = parent_tree_path.into_child(tree_model_sort);
    is_for_recurse_column_child(
        state,
        tree_view_column,
        child_tree_path,
        nix_store_res,
    )
}

pub fn is_event_button_for_recurse_column(
    state: &ui::State,
    event_button: gdk::EventButton,
    nix_store_res: &NixStoreRes,
) -> Option<NixQueryEntry> {
    event_button_to_parent_tree_path_column(state, event_button).and_then(
        |(parent_tree_path, tree_view_column)| {
            is_for_recurse_column_parent(
                state,
                tree_view_column,
                parent_tree_path,
                nix_store_res,
            )
        },
    )
}

pub fn nix_query_entry_for_event_button(
    state: &ui::State,
    event_button: gdk::EventButton,
    nix_store_res: &NixStoreRes,
) -> Option<NixQueryEntry> {
    let option_child_tree_path =
        event_button_to_child_tree_path(state, event_button.clone());

    let option_child_query_entry = option_child_tree_path
        .and_then(|x| x.nix_store_res_lookup(nix_store_res));

    option_child_query_entry
}
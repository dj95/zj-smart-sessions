use zellij_tile::prelude::*;

use rust_fuzzy_search::fuzzy_search_sorted;

#[derive(Default)]
pub struct SessionList {
    sessions: Vec<SessionInfo>,
    filtered_sessions: Vec<SessionInfo>,
    filtered_tabs: Vec<TabInfo>,
    selected_session_index: usize,
    selected_tab_index: usize,
    search_query: String,
    is_expanded: bool,
}

impl SessionList {
    pub fn new() -> Self {
        Self {
            sessions: vec![],
            filtered_sessions: vec![],
            filtered_tabs: vec![],
            selected_session_index: 0,
            selected_tab_index: 0,
            search_query: "".to_owned(),
            is_expanded: false,
        }
    }

    pub fn attach_selected(&mut self) {
        let session = self
            .filtered_sessions
            .get(self.selected_session_index)
            .unwrap();

        let tab = self.filtered_tabs.get(self.selected_tab_index).unwrap();

        tracing::debug!("session {} tab {}", session.name, tab.name);

        switch_session_with_focus(&session.name, Some(tab.position), None);
    }

    pub fn delete_selected(&mut self) {
        let session = self
            .filtered_sessions
            .get(self.selected_session_index)
            .unwrap();

        kill_sessions(&[&session.name]);
    }

    pub fn expand(&mut self) {
        self.is_expanded = true;
        self.selected_tab_index = 0;
    }

    pub fn shrink(&mut self) {
        self.is_expanded = false;
    }

    pub fn update_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        self.filter(&self.search_query.clone());
    }

    fn filter_tabs_for_selected_session(&mut self, search_query: &str) {
        tracing::debug!("selected_session_index {}", self.selected_session_index);

        let session = match self.sessions.get(self.selected_session_index) {
            Some(s) => s,
            None => return,
        };

        if search_query.is_empty() {
            self.filtered_tabs = session.tabs.clone();

            return;
        }

        let tab_names = session
            .tabs
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<&str>>();

        let result = fuzzy_search_sorted(search_query, &tab_names)
            .iter()
            .filter(|(_, score)| *score > 0.0)
            .map(|(c, _)| c.to_string())
            .collect::<Vec<String>>();

        self.filtered_tabs = result
            .into_iter()
            .map(|tn| session.tabs.iter().find(|t| t.name == tn).unwrap().clone())
            .collect();

        self.selected_tab_index = 0;
    }

    pub fn filter(&mut self, search_query: &str) {
        if self.sessions.is_empty() {
            return;
        }

        if search_query.is_empty() {
            self.filtered_sessions = self.sessions.clone();
            self.filter_tabs_for_selected_session(search_query);

            if self.is_expanded && !self.search_query.is_empty() {
                self.is_expanded = false;
            }

            self.search_query = search_query.to_owned();

            return;
        }

        self.search_query = search_query.to_owned();

        let mut search_query = search_query;
        let mut tab_query = "";
        if search_query.contains(' ') {
            let parts = search_query.split(' ').collect::<Vec<&str>>();

            search_query = parts[0];
            tab_query = parts[1];

            self.is_expanded = true;
        } else {
            self.is_expanded = false;
        }

        let session_names = self
            .sessions
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<&str>>();

        let result = fuzzy_search_sorted(search_query, &session_names)
            .iter()
            .filter(|(_, score)| *score > 0.0)
            .map(|(c, _)| c.to_string())
            .collect::<Vec<String>>();

        tracing::debug!("fuzzy_search result: {:?}", result);

        self.filtered_sessions = result
            .into_iter()
            .map(|sn| self.sessions.iter().find(|s| s.name == sn).unwrap().clone())
            .collect();

        self.selected_session_index = 0;

        self.filter_tabs_for_selected_session(tab_query);
    }

    pub fn select_next(&mut self) {
        if self.sessions.is_empty() {
            return;
        }

        if self.is_expanded {
            self.selected_tab_index = (self.selected_tab_index as i32 + 1)
                .rem_euclid(self.filtered_tabs.len() as i32)
                as usize;

            return;
        }

        self.selected_session_index = (self.selected_session_index as i32 + 1)
            .rem_euclid(self.filtered_sessions.len() as i32)
            as usize;
    }

    pub fn select_prev(&mut self) {
        if self.sessions.is_empty() {
            return;
        }

        if self.is_expanded {
            self.selected_tab_index = (self.selected_tab_index as i32 - 1)
                .rem_euclid(self.filtered_tabs.len() as i32)
                as usize;

            return;
        }

        self.selected_session_index = (self.selected_session_index as i32 - 1)
            .rem_euclid(self.filtered_sessions.len() as i32)
            as usize;
    }

    pub fn get_list(&self) -> Vec<NestedListItem> {
        let mut output: Vec<NestedListItem> = vec![];

        for (index, session) in self.filtered_sessions.clone().into_iter().enumerate() {
            let mut item = NestedListItem::new(format!(
                "{} ({} tabs, {} panes) [{} connected users]",
                &session.name,
                session.tabs.len(),
                session.panes.panes.len(),
                session.connected_clients,
            ))
            .color_range(0, 0..session.name.len())
            .color_range(1, session.name.len() + 2..session.name.len() + 3)
            .color_range(2, session.name.len() + 10..session.name.len() + 11)
            .color_range(0, session.name.len() + 20..session.name.len() + 21);

            if index == self.selected_session_index && !self.is_expanded {
                item = item.selected();
            }

            output.push(item);

            if index == self.selected_session_index && self.is_expanded {
                for (tab_index, tab) in self.filtered_tabs.clone().into_iter().enumerate() {
                    let mut tab_item = NestedListItem::new(format!(
                        "{} ({} panes)",
                        &tab.name,
                        session.panes.panes.len(),
                    ))
                    .color_range(1, 0..tab.name.len())
                    .color_range(2, tab.name.len() + 2..tab.name.len() + 3);

                    tab_item = tab_item.indent(1);

                    if tab_index == self.selected_tab_index {
                        tab_item = tab_item.selected();
                    }

                    output.push(tab_item);
                }
            }
        }

        output
    }
}

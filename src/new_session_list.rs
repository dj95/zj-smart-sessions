use std::collections::BTreeMap;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use zellij_tile::{
    prelude::SessionInfo,
    shim::{kill_sessions, run_command, switch_session_with_cwd, NestedListItem},
};

pub fn query_list(cmd: &str) {
    let command = commandline_parser(cmd);
    let context = BTreeMap::new();

    run_command(
        &command.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
        context,
    );
}

fn commandline_parser(input: &str) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();

    let special_chars = ['"', '\''];

    let mut found_special_char = '\0';
    let mut buffer = "".to_owned();
    let mut is_escaped = false;
    let mut is_in_group = false;

    for character in input.chars() {
        if is_escaped {
            is_escaped = false;
            buffer = format!("{}\\{}", buffer.to_owned(), character);
            continue;
        }

        if character == '\\' {
            is_escaped = true;
            continue;
        }

        if found_special_char == character && is_in_group {
            is_in_group = false;
            found_special_char = '\0';
            output.push(buffer.clone());
            "".clone_into(&mut buffer);
            continue;
        }

        if special_chars.contains(&character) && !is_in_group {
            is_in_group = true;
            found_special_char = character;
            continue;
        }

        if character == ' ' && !is_in_group {
            output.push(buffer.clone());
            "".clone_into(&mut buffer);
            continue;
        }

        buffer = format!("{}{}", buffer, character);
    }

    if !buffer.is_empty() {
        output.push(buffer.clone());
    }

    output
}

#[derive(Default)]
pub struct NewSessionList {
    list: Vec<String>,
    session_list: Vec<SessionInfo>,
    filtered_list: Vec<(String, Vec<usize>)>,
    filtered_list_len: usize,
    selected_item_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
    max_items: Option<usize>,
}

impl NewSessionList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_cache(&mut self) {
        let res = std::fs::read_to_string("/cache/store");

        tracing::debug!("cache {:?}", res);

        if let Ok(res) = res {
            self.list = res
                .split("\n")
                .map(|s| s.to_owned())
                .collect::<Vec<String>>();

            self.filter("");
        }
    }

    pub fn save_cache(&mut self) {
        let _ = std::fs::write("/cache/store", self.list.join("\n"));
    }

    pub fn update_list(&mut self, list: Vec<String>) {
        self.list = list;
        self.filter(&self.search_query.clone());
    }

    pub fn update_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.session_list = sessions;
    }

    pub fn has_list(&mut self) -> bool {
        !self.list.is_empty() && !self.session_list.is_empty()
    }

    pub fn create_or_attach(&mut self) {
        let item = self
            .filtered_list
            .get(self.selected_item_index)
            .unwrap()
            .0
            .strip_suffix("/")
            .unwrap();

        let name = item.split("/").last().unwrap().replace(".", "_");

        switch_session_with_cwd(Some(&name), Some(item.into()));
    }

    pub fn delete_selected(&mut self) {
        let item = self
            .filtered_list
            .get(self.selected_item_index)
            .unwrap()
            .0
            .strip_suffix("/")
            .unwrap();

        let name = item.split("/").last().unwrap().replace(".", "_");

        tracing::debug!("delete {}", name);

        if self.session_list.iter().any(|s| s.name == name) {
            kill_sessions(&[&name]);
        }
    }

    pub fn filter(&mut self, search_query: &str) {
        tracing::debug!("filter");
        if self.list.is_empty() {
            return;
        }

        self.search_query = search_query.to_owned();

        if search_query.is_empty() {
            self.filtered_list = self.list.iter().map(|i| (i.to_owned(), vec![])).collect();
            self.filtered_list_len = self.filtered_list.len();
            return;
        }

        let mut list = self
            .list
            .iter()
            .flat_map(|f| {
                self.matcher
                    .fuzzy_indices(f, search_query)
                    .map(|res| (f.to_owned(), res))
            })
            .collect::<Vec<(String, (i64, Vec<usize>))>>();

        list.sort_by_key(|i| std::cmp::Reverse(i.1 .0));

        self.filtered_list = list
            .iter()
            .map(|item| (item.0.to_owned(), item.1 .1.clone()))
            .collect::<Vec<(String, Vec<usize>)>>();

        self.filtered_list_len = self.filtered_list.len();

        if self.selected_item_index > self.filtered_list.len() {
            self.selected_item_index = self.filtered_list.len();
        }
    }

    pub fn select_next(&mut self) {
        if self.list.is_empty() {
            return;
        }
        tracing::debug!(
            "select_next {} {} {:?}",
            self.selected_item_index,
            self.filtered_list_len,
            self.max_items
        );

        if self.selected_item_index >= self.filtered_list.len() - 1 {
            self.selected_item_index = self.filtered_list.len() - 1;
            return;
        }

        if let Some(max) = self.max_items {
            self.selected_item_index = std::cmp::min(self.selected_item_index + 1, max);
            return;
        }

        self.selected_item_index = (self.selected_item_index as i32 + 1)
            .rem_euclid(self.filtered_list_len as i32) as usize;
    }

    pub fn select_prev(&mut self) {
        if self.list.is_empty() {
            return;
        }

        self.selected_item_index = self.selected_item_index.saturating_sub(1);
    }

    fn list_window(&mut self, height: usize) -> Vec<(String, Vec<usize>)> {
        if self.filtered_list.is_empty() {
            return vec![];
        }

        if self.filtered_list.len() < height {
            return self.filtered_list.clone();
        }

        self.filtered_list[0..=height].to_owned()
    }

    pub fn get_list(&mut self, height: usize) -> Vec<NestedListItem> {
        self.max_items = Some(height);

        let mut output: Vec<NestedListItem> = vec![];

        let list_window = self.list_window(height);
        tracing::debug!("selected {}", self.selected_item_index);

        for (index, match_name) in list_window.into_iter().enumerate() {
            let (match_name, indice) = match_name.clone();
            let name = match_name
                .strip_suffix("/")
                .unwrap_or(&match_name)
                .split("/")
                .last()
                .unwrap()
                .replace(".", "_");

            tracing::debug!("name {}", name);

            let mut item = NestedListItem::new(&name)
                .color_range(0, 0..name.len())
                .color_indices(1, indice);

            if !self.session_list.is_empty() {
                if let Some(session) = self.session_list.iter().find(|s| s.name == name.clone()) {
                    item = NestedListItem::new(format!(
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
                }
            }

            if index == std::cmp::min(self.selected_item_index, height) {
                item = item.selected();
            }

            output.push(item);
        }

        output
    }
}

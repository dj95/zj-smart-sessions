use zellij_tile::prelude::*;
use zj_smart_sessions::{
    new_session_list::{query_list, NewSessionList},
    session_list::SessionList,
};

use std::collections::BTreeMap;

#[cfg(not(test))]
register_plugin!(State);

#[cfg(feature = "tracing")]
fn init_tracing() {
    use std::fs::File;
    use std::sync::Arc;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let file = File::create("/host/.zj-smart-sessions.log");
    let file = match file {
        Ok(file) => file,
        Err(error) => panic!("Error: {:?}", error),
    };
    let debug_log = tracing_subscriber::fmt::layer().with_writer(Arc::new(file));

    tracing_subscriber::registry().with(debug_log).init();

    tracing::info!("tracing initialized");
}

#[derive(Default)]
struct State {
    hidden: bool,
    session_list: SessionList,
    new_session_list: NewSessionList,
    search_query: String,
    find_command: Option<String>,
    queried_files: bool,
}

impl ZellijPlugin for State {
    fn load(&mut self, config: BTreeMap<String, String>) {
        #[cfg(feature = "tracing")]
        init_tracing();

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::SessionUpdate,
            EventType::Key,
            EventType::RunCommandResult,
        ]);

        self.hidden = false;
        self.search_query = "".to_owned();
        self.session_list = SessionList::new();
        self.new_session_list = NewSessionList::new();
        self.new_session_list.load_cache();
        self.find_command = config.get("find_command").map(|i| i.to_owned());
    }

    fn update(&mut self, event: Event) -> bool {
        let is_new_session = self.find_command.is_some();
        if !self.queried_files {
            if let Some(find_command) = self.find_command.clone() {
                tracing::debug!("fetching files with: {find_command}");
                query_list(&find_command);
                self.queried_files = true;
            }
        }

        let mut should_render = false;

        match event {
            Event::PermissionRequestResult(_) => {
                should_render = true;
            }
            Event::RunCommandResult(_code, stdout, _stderr, _ctx) => {
                let stdout = String::from_utf8(stdout.clone())
                    .expect("")
                    .split('\n')
                    .map(|s| s.to_owned())
                    .collect::<Vec<String>>();

                tracing::debug!("got result {:?}", stdout);
                tracing::debug!("got result");

                self.new_session_list.update_list(stdout);
                self.new_session_list.save_cache();
                should_render = true;
            }
            Event::SessionUpdate(info, _foo) => {
                self.session_list.update_sessions(info.clone());
                self.new_session_list.update_sessions(info);
                should_render = true;
            }
            Event::Key(key) => match key.bare_key {
                BareKey::Enter => {
                    if is_new_session {
                        self.new_session_list.create_or_attach();
                    } else {
                        self.session_list.attach_selected();
                    }
                    close_self();
                }
                BareKey::Delete => {
                    if is_new_session {
                        self.new_session_list.delete_selected();
                    } else {
                        self.session_list.delete_selected();
                    }
                }
                BareKey::Backspace => {
                    if self.search_query.is_empty() {
                        if is_new_session {
                            self.new_session_list.filter(&self.search_query);
                        }

                        return false;
                    }

                    self.search_query = self
                        .search_query
                        .chars()
                        .take(self.search_query.len() - 1)
                        .collect();

                    if is_new_session {
                        self.new_session_list.filter(&self.search_query);
                    }

                    should_render = true;
                }
                BareKey::Down => {
                    if is_new_session {
                        self.new_session_list.select_next();
                    } else {
                        self.session_list.select_next();
                    }
                    should_render = true
                }
                BareKey::Up => {
                    if is_new_session {
                        self.new_session_list.select_prev();
                    } else {
                        self.session_list.select_prev();
                    }
                    should_render = true;
                }
                BareKey::Left => {
                    self.session_list.shrink();
                    should_render = true;
                }
                BareKey::Right => {
                    self.session_list.expand();
                    should_render = true;
                }
                BareKey::Esc => {
                    close_self();
                }
                BareKey::Char(' ') => {
                    self.search_query = self.search_query.clone() + " ";
                    if is_new_session {
                        self.new_session_list.filter(&self.search_query);
                    }
                    should_render = true
                }
                _ => {
                    self.search_query = self.search_query.clone() + &key.to_string();
                    if is_new_session {
                        self.new_session_list.filter(&self.search_query);
                    }
                    should_render = true
                }
            },
            _ => {}
        }

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if self.find_command.is_some() {
            print_text_with_coordinates(
                Text::new(format!("Search: {}_", self.search_query)).color_range(2, 0..7),
                0,
                0,
                Some(cols),
                None,
            );

            let list = self.new_session_list.get_list(rows - 5);
            print_nested_list_with_coordinates(list, 0, 2, Some(cols), None);

            print_text_with_coordinates(
                Text::new("Attach: <Enter> // Delete: <Del> // Exit: <Esc>")
                    .color_range(3, 8..15)
                    .color_range(3, 27..32)
                    .color_range(3, 42..47),
                0,
                rows - 1,
                Some(cols),
                None,
            );
            return;
        }

        tracing::debug!("search query: {}", self.search_query);

        print_text_with_coordinates(
            Text::new(format!("Search: {}_", self.search_query)).color_range(2, 0..7),
            0,
            0,
            Some(cols),
            None,
        );

        self.session_list.filter(&self.search_query);

        let list = self.session_list.get_list();
        print_nested_list_with_coordinates(list, 0, 2, Some(cols), None);

        print_text_with_coordinates(
            Text::new("Attach: <Enter> // Delete: <Del> // Exit: <Esc>")
                .color_range(3, 8..15)
                .color_range(3, 27..32)
                .color_range(3, 42..47),
            0,
            rows - 1,
            Some(cols),
            None,
        );
    }
}

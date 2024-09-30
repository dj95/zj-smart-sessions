use zellij_tile::prelude::*;
use zj_smart_sessions::session_list::SessionList;

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
    search_query: String,
}

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        #[cfg(feature = "tracing")]
        init_tracing();

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::SessionUpdate,
            EventType::Key,
        ]);

        self.hidden = false;
        self.search_query = "".to_owned();
        self.session_list = SessionList::new();
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        match event {
            Event::SessionUpdate(info, _foo) => {
                self.session_list.update_sessions(info);
                should_render = true;
            }
            Event::Key(key) => match key {
                Key::Char('\n') => {
                    self.session_list.attach_selected();
                    close_self();
                }
                Key::Backspace => {
                    if self.search_query.is_empty() {
                        return false;
                    }

                    self.search_query = self
                        .search_query
                        .chars()
                        .take(self.search_query.len() - 1)
                        .collect();

                    should_render = true;
                }
                Key::Down => {
                    self.session_list.select_next();
                    should_render = true
                }
                Key::Up => {
                    self.session_list.select_prev();
                    should_render = true;
                }
                Key::Left => {
                    self.session_list.shrink();
                    should_render = true;
                }
                Key::Right => {
                    self.session_list.expand();
                    should_render = true;
                }
                Key::Esc => {
                    close_self();
                }
                Key::Char(' ') => {
                    self.search_query = self.search_query.clone() + " ";
                    should_render = true
                }
                _ => {
                    self.search_query = self.search_query.clone() + &key.to_string();
                    should_render = true
                }
            },
            _ => {}
        }

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
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

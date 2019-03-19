#[macro_use]
extern crate papyrus;

use azul::prelude::*;
use linefeed::memory::MemoryTerminal;
use papyrus::widgets::repl_terminal::*;

struct MyApp {
    repl_term: ReplTerminalState,
}

impl GetReplTerminal for MyApp {
    fn repl_term(&mut self) -> &mut ReplTerminalState {
        &mut self.repl_term
    }
}

impl Layout for MyApp {
    fn layout(&self, info: LayoutInfo<Self>) -> Dom<Self> {
        Dom::div()
            .with_child(ReplTerminal::new(info.window, &self.repl_term, &self).dom(&self.repl_term))
    }
}

fn main() {
    let term = MemoryTerminal::new();

    let repl = repl_with_term!(term.clone());

    let app = {
        App::new(
            MyApp {
                repl_term: ReplTerminalState::new(repl),
            },
            AppConfig {
                enable_logging: Some(LevelFilter::Error),
                log_file_path: Some("debug.log".to_string()),
                ..Default::default()
            },
        )
    };
    let window = if cfg!(debug_assertions) {
        Window::new_hot_reload(
            WindowCreateOptions::default(),
            css::hot_reload_override_native(
                "styles/test.css",
                std::time::Duration::from_millis(1000),
            ),
        )
        .unwrap()
    // Window::new(WindowCreateOptions::default(), css::native()).unwrap()
    } else {
        Window::new(WindowCreateOptions::default(), css::native()).unwrap()
    };

    app.run(window).unwrap();
}

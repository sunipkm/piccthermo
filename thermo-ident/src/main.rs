use cursive::views;

fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();

    // Create a new Cursive instance.
    let mut siv = cursive::default();

    // Clear the global callbacks for Ctrl-C to prevent the default behavior.
    siv.clear_global_callbacks(cursive::event::Event::CtrlChar('c'));

    // Set a custom callback for Ctrl-C to show quit confirmation dialog.
    siv.set_on_pre_event(cursive::event::Event::CtrlChar('c'), |s| {
        add_quit_layer(s);
    });

    // Set a custom callback for 'q' to show quit confirmation dialog.
    siv.set_on_pre_event(cursive::event::Event::Char('q'), |s| {
        add_quit_layer(s);
    });

    // Add a global callback for '~' to toggle the debug console.
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);


    siv.add_layer(views::Dialog::text("Try pressing Ctrl-C!"));

    siv.run();
}

fn add_quit_layer(s: &mut cursive::Cursive) {
    s.add_layer(
        views::Dialog::text("Do you want to quit?")
            .button("Yes", |s| s.quit())
            .button("No", |s| {
                s.pop_layer();
            }),
    )
}

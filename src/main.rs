/*!
 * Myxer is a GTK based pulse audio volume mixer.
 * This module serves as the main entry point to the application.
 */

#![allow(clippy::tabs_in_doc_comments)]

mod card;
mod meter;
mod pulse;
mod shared;
mod window;

use gdk::prelude::{ApplicationExt, ApplicationExtManual};
use pulse::Pulse;
use shared::Shared;
use std::time::Duration;
use window::Myxer;

/**
 * Attempts to start the application.
 * The attempt will abort if another instance is already live, this is
 * because GTK attempts to share the pulse manager between instances,
 * which doesn't work with the message consumption model that is currently
 * in-place. Presumably, there is a way to make both instances have completely
 * independent memory, but I was unable to find it.
 */

fn main() {
    let pulse = Shared::new(Pulse::new());

    let app = gtk::Application::new(Some("com.aurailus.myxer"), Default::default());

    let pulse_clone = pulse.clone();
    app.connect_activate(|app| drop(app.register::<gio::Cancellable>(None)));
    app.connect_startup(move |app| activate(app, &pulse_clone));
    app.run();

    pulse.borrow_mut().cleanup();
}

/**
 * Called by GTK when the application has initialized. Creates the main Myxer
 * instance, which controls the visible window, and handles the update loop.
 */

fn activate(app: &gtk::Application, pulse: &Shared<Pulse>) {
    let mut myxer = Myxer::new(app, pulse);

    glib::timeout_add_local(Duration::from_millis(1000 / 30), move || {
        myxer.update();
        glib::Continue(true)
    });
}

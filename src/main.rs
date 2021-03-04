use gio::prelude::*;

mod card;
mod myxer;
mod style;
mod meter;
mod about;
mod shared;
mod profiles;
mod pulse_controller;

use myxer::Myxer;
use shared::Shared;
use pulse_controller::PulseController;

fn main() {
	let pulse = Shared::new(PulseController::new());

	let app = gtk::Application::new(Some("com.aurailus.myxer"), Default::default())
		.expect("Failed to initialize GTK application.");

	{
		let pulse = pulse.clone();
		app.connect_activate(move |app| activate(app, &pulse));
		app.run(&[]);
	}

	pulse.borrow_mut().cleanup();
}

fn activate(app: &gtk::Application, pulse: &Shared<PulseController>) {
	let mut myxer = Myxer::new(app, pulse);

	glib::timeout_add_local(1000 / 30, move || {
		myxer.update();
		glib::Continue(true)
	});
}

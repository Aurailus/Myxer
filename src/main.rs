use gio::prelude::*;

mod card;
mod meter;
mod pulse;
mod window;
mod shared;

use pulse::Pulse;
use window::Myxer;
use shared::Shared;

fn main() {
	let pulse = Shared::new(Pulse::new());
	
	let app = gtk::Application::new(Some("com.aurailus.myxer"), Default::default())
		.expect("Failed to initialize GTK application.");

	{
		let pulse = pulse.clone();
		app.connect_activate(|app| drop(app.register::<gio::Cancellable>(None)));
		app.connect_startup(move |app| activate(app, &pulse));
		app.run(&[]);
	}

	pulse.borrow_mut().cleanup();
}

fn activate(app: &gtk::Application, pulse: &Shared<Pulse>) {
	let mut myxer = Myxer::new(app, pulse);

	glib::timeout_add_local(1000 / 30, move || {
		myxer.update();
		glib::Continue(true)
	});
}

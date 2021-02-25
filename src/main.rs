#[macro_use]
extern crate glib;
extern crate gtk;
extern crate gio;

mod shared;
mod pulse;
#[path = "./widget/volume.rs"]
mod volume;
#[path = "./widget/notebook.rs"]
mod notebook;

use shared::Shared;
use std::sync::mpsc::channel;

use gtk::prelude::*;
use gio::prelude::*;

use volume::Volume;
use volume::VolumeExt;
use notebook::Notebook;
use crate::pulse::{ PulseController, PulseStore, PulseTx };

fn main() {
	let pulse = Shared::new(PulseController::new());
	pulse.borrow_mut().connect();

	let app = gtk::Application::new(Some("com.aurailus.vmix"), Default::default())
		.expect("Failed to initialize GTK application.");
		
	let pulse_shr = pulse.clone();
	app.connect_activate(move |app| activate(app, pulse_shr.clone()));
	app.run(&[]);

	pulse.borrow_mut().cleanup();
}

fn activate(app: &gtk::Application, pulse_shr: Shared<PulseController>) {
	let window = gtk::ApplicationWindow::new(app);
	window.set_title("VMix");
	window.set_resizable(false);
	window.set_default_size(490, 320);
	window.set_border_width(0);
	
	let (tx, rx) = channel::<PulseTx>();
	let mut pulse = pulse_shr.borrow_mut();
	pulse.subscribe(tx);


	let mut store = PulseStore::new();
	// app.connect_shutdown(move |_| pulse.cleanup());

	glib::timeout_add_local(100, move || {
		let mut changed = false;
		loop {
			let res = rx.try_recv();
			match res {
				Ok(res) => match res {
					PulseTx::INPUT(index, data) => match data {
						Some(input) => { store.inputs.insert(index, input); },
						None => { store.inputs.remove(&index); }
					},
					PulseTx::SINK(index, data) => match data {
						Some(sink) => { store.sinks.insert(index, sink); },
						None => { store.inputs.remove(&index); }
					},
					PulseTx::END => changed = true
				},
				_ => break
			}
		}

		if changed {
			println!("{:?}", store);
		}
		glib::Continue(true)
	});

	let style = include_str!("./style.css");
	let provider = gtk::CssProvider::new();
	provider.load_from_data(style.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK css provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

	let outer_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	outer_container.set_border_width(4);
	let system_volume = Volume::new();
	system_volume.set_system();
	system_volume.get_style_context().add_class("bordered");
	outer_container.pack_start(&system_volume, false, false, 0);

	let scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	scroller.get_style_context().add_class("bordered");
	outer_container.pack_start(&scroller, true, true, 0);

	let inner_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	scroller.add(&inner_container);

	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);

	let mut notebook = Notebook::new();
	notebook.add_tab("Playback", outer_container.upcast());
	notebook.add_tab("Recording", gtk::Box::new(gtk::Orientation::Vertical, 0).upcast());

	window.add(&notebook.widget);

	window.show_all();
}

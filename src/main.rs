mod shared;
mod pulse;
#[path = "./widget/meter.rs"]
mod meter;
#[path = "./widget/notebook.rs"]
mod notebook;

extern crate gtk;
extern crate gio;

#[macro_use]
extern crate slice_as_array;

// use std::borrow::BorrowMut;
use std::collections::HashMap;
// use std::sync::mpsc::channel;

use shared::Shared;

use gtk::prelude::*;
use gio::prelude::*;

use notebook::Notebook;
use meter::{ Meter, SinkMeter };
use crate::pulse::{ PulseController };

fn main() {
	let pulse = Shared::new(PulseController::new());
	pulse.borrow_mut().connect();

	let app = gtk::Application::new(Some("com.aurailus.vmix"), Default::default())
		.expect("Failed to initialize GTK application.");
		
	let pulse_shr = pulse.clone();
	app.connect_activate(move |app| activate(app, pulse_shr.clone()));
	app.run(&[]);

	// pulse.borrow_mut().cleanup();
}

fn activate(app: &gtk::Application, pulse_shr: Shared<PulseController>) {
	let window = gtk::ApplicationWindow::new(app);
	window.set_title("Volume Mixer");
	window.set_border_width(0);
	window.set_resizable(false);
	window.set_default_size(530, 320);
	window.set_icon_name(Some("multimedia-volume-control"));

	{
		let mut pulse = pulse_shr.borrow_mut();
		pulse.subscribe();
	}

	// app.connect_shutdown(move |_| pulse.cleanup());

	// Include styles

	let style = include_str!("./style.css");
	let provider = gtk::CssProvider::new();
	provider.load_from_data(style.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK css provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

	// Create container

	let outer_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	outer_container.set_border_width(4);

	// Add sink meter

	let mut sink_meter = SinkMeter::new();
	sink_meter.widget.get_style_context().add_class("outer");
	sink_meter.widget.get_style_context().add_class("bordered");
	outer_container.pack_start(&sink_meter.widget, false, false, 0);

	// Add sink inputs scroller

	let scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	scroller.get_style_context().add_class("bordered");
	outer_container.pack_start(&scroller, true, true, 0);

	let inner_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	scroller.add(&inner_container);

	let mut system_meter = SinkMeter::new();
	system_meter.set_name_and_icon("System Sounds", "multimedia-volume-control");
	system_meter.set_volume(65535);
	inner_container.pack_start(&system_meter.widget, false, false, 0);

	let meters: Shared<HashMap<u32, SinkMeter>> = Shared::new(HashMap::new());
	
	glib::timeout_add_local(50, move || {
		let mut pulse = pulse_shr.borrow_mut();
		if pulse.update() {
			let sink_opt = pulse.sinks.iter().next();
			if sink_opt.is_some() {
				let sink = sink_opt.unwrap().1;
				sink_meter.set_name_and_icon(sink.data.name.as_str(), "audio-headphones");
				sink_meter.set_volume(sink.data.volume.0);
				sink_meter.set_muted(sink.data.muted);
				sink_meter.set_peak_volume(sink.peak);
			}

			let mut meters = meters.borrow_mut();
			for (index, input) in pulse.sink_inputs.iter() {
				let meter = meters.entry(*index).or_insert(SinkMeter::new());
				meter.set_name_and_icon(input.data.name.as_str(), input.data.icon.as_str());
				meter.set_volume(input.data.volume.0);
				meter.set_muted(input.data.muted);
				meter.set_peak_volume(input.peak);

				if meter.widget.get_parent().is_none() {
					inner_container.pack_start(&meter.widget, false, false, 0);
				}
			}

			meters.retain(|index, meter| {
				let keep = pulse.sink_inputs.contains_key(index);
				if !keep { inner_container.remove(&meter.widget); }
				keep
			});

			inner_container.show_all();
		}

		glib::Continue(true)
	});

	let mut notebook = Notebook::new();
	notebook.add_tab("Playback", outer_container.upcast());
	notebook.add_tab("Recording", gtk::Box::new(gtk::Orientation::Vertical, 0).upcast());

	window.add(&notebook.widget);

	window.show_all();
}

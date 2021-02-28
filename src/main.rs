mod shared;
mod pulse;
mod pulse_data;
#[path = "./widget/meter.rs"]
mod meter;
#[path = "./widget/notebook.rs"]
mod notebook;

extern crate gtk;
extern crate gio;
#[macro_use] extern crate slice_as_array;

use std::collections::HashMap;

use shared::Shared;

use gtk::prelude::*;
use gio::prelude::*;

use notebook::Notebook;
use meter::{ Meter, StreamMeter };
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

	// let header = gtk::HeaderBar::new();
	// header.set_show_close_button(true);
	// header.set_title(Some("Volume Mixer"));
	// header.set_decoration_layout(Some("icon:minimize,close"));
	// // header.set_has_subtitle(false);
	
	// let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	// navigation.get_style_context().add_class("linked");
	// let playback_btn = gtk::ToggleButton::with_label("Playback");
	// // playback_btn.set_backdrop(true);
	// let recording_btn = gtk::ToggleButton::with_label("Recording");
	// navigation.pack_end(&playback_btn, false, false, 0);
	// navigation.pack_end(&recording_btn, false, false, 0);
	// header.pack_start(&navigation);

	// window.set_titlebar(Some(&header));


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

	let playback = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	playback.set_border_width(4);

	let mut sink_meter = StreamMeter::new();
	sink_meter.widget.get_style_context().add_class("outer");
	sink_meter.widget.get_style_context().add_class("bordered");
	playback.pack_start(&sink_meter.widget, false, false, 0);

	let playback_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	playback_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	playback_scroller.get_style_context().add_class("bordered");
	playback.pack_start(&playback_scroller, true, true, 0);

	let playback_inner_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	playback_scroller.add(&playback_inner_box);

	let recording = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	recording.set_border_width(4);

	let mut source_meter = StreamMeter::new();
	source_meter.widget.get_style_context().add_class("outer");
	source_meter.widget.get_style_context().add_class("bordered");
	recording.pack_start(&source_meter.widget, false, false, 0);

	let recording_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	recording_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	recording_scroller.get_style_context().add_class("bordered");
	recording.pack_start(&recording_scroller, true, true, 0);

	let recording_inner_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	recording_scroller.add(&recording_inner_box);

	let mut system_meter = StreamMeter::new();
	system_meter.set_name_and_icon("System Sounds", "multimedia-volume-control");
	system_meter.set_volume(65535);
	playback_inner_box.pack_start(&system_meter.widget, false, false, 0);

	let sink_meters: Shared<HashMap<u32, StreamMeter>> = Shared::new(HashMap::new());
	let source_meters: Shared<HashMap<u32, StreamMeter>> = Shared::new(HashMap::new());
	
	glib::timeout_add_local(1000 / 10, move || {
		let mut pulse = pulse_shr.borrow_mut();
		if pulse.update() {
			let sink_opt = pulse.sinks.iter().next();
			if sink_opt.is_some() {
				let sink = sink_opt.unwrap().1;
				sink_meter.set_name_and_icon(sink.data.description.as_str(), "audio-headphones");
				sink_meter.set_volume(sink.data.volume.0);
				sink_meter.set_muted(sink.data.muted);
				sink_meter.set_peak_volume(sink.peak);
				// sink_meter.refresh();
			}

			let mut meters = sink_meters.borrow_mut();
			for (index, input) in pulse.sink_inputs.iter() {
				let meter = meters.entry(*index).or_insert(StreamMeter::new());
				meter.set_name_and_icon(input.data.name.as_str(), input.data.icon.as_str());
				meter.set_volume(input.data.volume.0);
				meter.set_muted(input.data.muted);
				meter.set_peak_volume(input.peak);
				// meter.refresh();

				if meter.widget.get_parent().is_none() {
					playback_inner_box.pack_start(&meter.widget, false, false, 0);
				}
			}

			meters.retain(|index, meter| {
				let keep = pulse.sink_inputs.contains_key(index);
				if !keep { playback_inner_box.remove(&meter.widget); }
				keep
			});

			let mut meters = source_meters.borrow_mut();
			for (index, output) in pulse.source_outputs.iter() {
				let meter = meters.entry(*index).or_insert(StreamMeter::new());
				meter.set_name_and_icon(output.data.name.as_str(), output.data.icon.as_str());
				meter.set_volume(output.data.volume.0);
				meter.set_muted(output.data.muted);
				meter.set_peak_volume(output.peak);
				meter.refresh();

				if meter.widget.get_parent().is_none() {
					recording_inner_box.pack_start(&meter.widget, false, false, 0);
				}
			}

			meters.retain(|index, meter| {
				let keep = pulse.source_outputs.contains_key(index);
				if !keep { recording_inner_box.remove(&meter.widget); }
				keep
			});

			playback_inner_box.show_all();
			recording_inner_box.show_all();
		}

		glib::Continue(true)
	});

	let mut notebook = Notebook::new();
	notebook.add_tab("Playback", playback.upcast());
	notebook.add_tab("Recording", recording.upcast());

	window.add(&notebook.widget);

	window.show_all();
}

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

use meter::{ Meter, StreamMeter };
use crate::pulse::{ PulseController };

struct Meters {
	pub sink: StreamMeter,
	pub sink_inputs: HashMap<u32, StreamMeter>,
	pub sink_inputs_box: gtk::Box,
	
	pub source: StreamMeter,
	pub source_outputs: HashMap<u32, StreamMeter>,
	pub source_outputs_box: gtk::Box
}

impl Meters {
	pub fn new() -> Self {
		let mut sink = StreamMeter::new();
		sink.widget.get_style_context().add_class("outer");
		sink.widget.get_style_context().add_class("bordered");

		let source = StreamMeter::new();
		source.widget.get_style_context().add_class("outer");
		source.widget.get_style_context().add_class("bordered");

		Meters {
			sink, source,
			sink_inputs: HashMap::new(),
			sink_inputs_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
			source_outputs: HashMap::new(),
			source_outputs_box: gtk::Box::new(gtk::Orientation::Horizontal, 0)
		}
	}
}

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
	
	// Window & Header

	let window = gtk::ApplicationWindow::new(app);
	window.set_title("Volume Mixer");
	window.set_border_width(0);
	window.set_resizable(false);
	window.set_default_size(530, 320);
	window.set_icon_name(Some("multimedia-volume-control"));

	let stack = gtk::Stack::new();
	let stack_switcher = gtk::StackSwitcher::new();
	stack_switcher.set_stack(Some(&stack));

	let header = gtk::HeaderBar::new();
	header.set_show_close_button(true);

	let title = gtk::Label::new(Some("Volume Mixer"));
	title.get_style_context().add_class("title");
	header.pack_start(&title);
	header.set_decoration_layout(Some("icon:minimize,close"));

	let preferences_btn = gtk::Button::from_icon_name(Some("applications-system-symbolic"), gtk::IconSize::SmallToolbar);
	preferences_btn.get_style_context().add_class("titlebutton");
	preferences_btn.set_widget_name("preferences");
	preferences_btn.set_can_focus(false);
	header.pack_end(&preferences_btn);

	let preferences_popover = gtk::Popover::new(Some(&preferences_btn));
	let aaa = gtk::Label::new(Some("aadawdwadaa\ndwahdawhd\ndawdad"));
	preferences_popover.add(&aaa);

	let preferences_popover_clone = preferences_popover.clone();
	preferences_btn.connect_clicked(move |_| preferences_popover_clone.show_all());
	header.set_custom_title(Some(&stack_switcher));

	window.set_titlebar(Some(&header));

	// Setup Pulse

	{
		let mut pulse = pulse_shr.borrow_mut();
		pulse.subscribe();
	}

	// Include styles

	let style = include_str!("./style.css");
	let provider = gtk::CssProvider::new();
	provider.load_from_data(style.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK css provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);


	// Add Meters & Elements

	let meters = Shared::new(Meters::new());

	let playback = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	playback.pack_start(&meters.borrow().sink.widget, false, false, 0);
	playback.set_border_width(4);

	let playback_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	playback_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	playback_scroller.get_style_context().add_class("bordered");
	playback.pack_start(&playback_scroller, true, true, 0);
	playback_scroller.add(&meters.borrow().sink_inputs_box);

	let recording = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	recording.pack_start(&meters.borrow().source.widget, false, false, 0);
	recording.set_border_width(4);

	let recording_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	recording_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	recording_scroller.get_style_context().add_class("bordered");
	recording.pack_start(&recording_scroller, true, true, 0);
	recording_scroller.add(&meters.borrow().source_outputs_box);

	let mut system_meter = StreamMeter::new();
	system_meter.set_name_and_icon("System Sounds", "multimedia-volume-control");
	system_meter.set_volume(65535);
	meters.borrow().sink_inputs_box.pack_start(&system_meter.widget, false, false, 0);

	glib::timeout_add_local(1000 / 30, move || {
		let meters_shr = meters.clone();
		let pulse_shr = pulse_shr.clone();
		update(pulse_shr, meters_shr);
		glib::Continue(true)
	});

	// let mut notebook = Notebook::new();
	// notebook.add_tab("Playback", playback.upcast());
	// notebook.add_tab("Recording", recording.upcast());
	stack.add_titled(&playback, "playback", "Output");
	stack.add_titled(&recording, "recording", "Input");

	window.add(&stack);

	// window.add(&notebook.widget);

	window.show_all();
}

fn update(pulse_shr: Shared<PulseController>, meters_shr: Shared<Meters>) {

	if pulse_shr.borrow_mut().update() {
		let pulse = pulse_shr.borrow();
		let mut meters = meters_shr.borrow_mut();

		let sink_opt = pulse.sinks.iter().next();
		if sink_opt.is_some() {
			let sink = sink_opt.unwrap().1;
			meters.sink.set_name_and_icon(sink.data.description.as_str(), "audio-headphones");
			meters.sink.set_volume(sink.data.volume.0);
			meters.sink.set_muted(sink.data.muted);
			meters.sink.set_peak_volume(sink.peak);
			meters.sink.refresh();
		}

		for (index, input) in pulse.sink_inputs.iter() {
			let sink_inputs_box = meters.sink_inputs_box.clone();
			let pulse_shr = pulse_shr.clone();

			let meter = meters.sink_inputs.entry(*index).or_insert({
				let s = StreamMeter::new();
				let index: u32 = *index;
				s.widgets.scale.connect_change_value(move |_, _, value| {
					let pulse = pulse_shr.borrow_mut();
					pulse.set_sink_input_volume(index, value as u32);
					gtk::Inhibit(false)
				});
				s
			});

			meter.set_name_and_icon(input.data.name.as_str(), input.data.icon.as_str());
			meter.set_volume(input.data.volume.0);
			meter.set_muted(input.data.muted);
			meter.set_peak_volume(input.peak);
			meter.refresh();
			
			if meter.widget.get_parent().is_none() {
				sink_inputs_box.pack_start(&meter.widget, false, false, 0);
			}
		}

		let sink_inputs_box = meters.sink_inputs_box.clone();
		meters.sink_inputs.retain(|index, meter| {
			let keep = pulse.sink_inputs.contains_key(index);
			if !keep { sink_inputs_box.remove(&meter.widget); }
			keep
		});

		for (index, output) in pulse.source_outputs.iter() {
			let source_outputs_box = meters.source_outputs_box.clone();
			
			let meter = meters.source_outputs.entry(*index).or_insert(StreamMeter::new());
			meter.set_name_and_icon(output.data.name.as_str(), output.data.icon.as_str());
			meter.set_volume(output.data.volume.0);
			meter.set_muted(output.data.muted);
			meter.set_peak_volume(output.peak);
			meter.refresh();

			if meter.widget.get_parent().is_none() {
				source_outputs_box.pack_start(&meter.widget, false, false, 0);
			}
		}

		let source_outputs_box = meters.source_outputs_box.clone();
		meters.source_outputs.retain(|index, meter| {
			let keep = pulse.source_outputs.contains_key(index);
			if !keep { source_outputs_box.remove(&meter.widget); }
			keep
		});

		meters.sink_inputs_box.show_all();
		meters.source_outputs_box.show_all();
	}
}

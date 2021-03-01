mod shared;
mod pulse;
mod pulse_data;
#[path = "./widget/meter.rs"]
mod meter;
#[path = "./widget/notebook.rs"]
mod notebook;

extern crate gtk;
extern crate gio;
#[macro_use]
extern crate slice_as_array;

use std::collections::HashMap;

use shared::Shared;

use gtk::prelude::*;
use gio::prelude::*;
use gdk::WindowHints as WH;

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
		let sink = StreamMeter::new();
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
	let pulse_shr = Shared::new(PulseController::new());

	let app = gtk::Application::new(Some("com.aurailus.vmix"), Default::default())
		.expect("Failed to initialize GTK application.");
		
	let pulse = pulse_shr.clone();
	app.connect_activate(move |app| activate(app, pulse.clone()));
	app.run(&[]);

	pulse_shr.borrow_mut().cleanup();
}

fn activate(app: &gtk::Application, pulse_shr: Shared<PulseController>) {

	// Basic Structure

	let window = gtk::ApplicationWindow::new(app);
	window.set_title("Volume Mixer");
	window.set_icon_name(Some("multimedia-volume-control"));

	let geom = gdk::Geometry {
		min_width: 580, min_height: 400,
		max_width: 10000, max_height: 400,
		base_width: -1, base_height: -1,
		width_inc: -1, height_inc: -1,
		min_aspect: 0.0, max_aspect: 0.0,
		win_gravity: gdk::Gravity::Center
	};

	window.set_geometry_hints::<gtk::ApplicationWindow>(None, Some(&geom), WH::MIN_SIZE | WH::MAX_SIZE);

	let style = include_str!("./style.css");
	let provider = gtk::CssProvider::new();
	provider.load_from_data(style.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK css provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

	let stack = gtk::Stack::new();
	let stack_switcher = gtk::StackSwitcher::new();
	stack_switcher.set_stack(Some(&stack));

	let header = gtk::HeaderBar::new();
	header.set_show_close_button(true);
	header.set_custom_title(Some(&stack_switcher));

	let title = gtk::Label::new(Some("Volume Mixer"));
	title.get_style_context().add_class("title");
	header.pack_start(&title);
	header.set_decoration_layout(Some("icon:minimize,close"));

	let prefs_button = gtk::Button::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::SmallToolbar);
	prefs_button.get_style_context().add_class("titlebutton");
	prefs_button.set_widget_name("preferences");
	prefs_button.set_can_focus(false);
	header.pack_end(&prefs_button);

	let prefs_popover = gtk::PopoverMenu::new();
	prefs_popover.set_pointing_to(&gtk::Rectangle { x: 12, y: 32, width: 2, height: 2 });
	prefs_popover.set_relative_to(Some(&prefs_button));
	prefs_popover.set_border_width(8);
	let prefs_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
	prefs_popover.add(&prefs_box);

	let prefs_preferences = gtk::ModelButton::new();
	prefs_preferences.set_property_text(Some("Preferences"));
	prefs_box.add(&prefs_preferences);

	let prefs_help = gtk::ModelButton::new();
	prefs_help.set_property_text(Some("Help"));
	prefs_help.set_action_name(Some("app.help"));
	prefs_box.add(&prefs_help);

	let prefs_about = gtk::ModelButton::new();
	prefs_about.set_property_text(Some("About VMix"));
	prefs_about.connect_clicked(|_| show_about());
	prefs_box.add(&prefs_about);

	prefs_popover.show_all();

	let prefs_popover_clone = prefs_popover.clone();
	prefs_button.connect_clicked(move |_| prefs_popover_clone.popup());

	window.set_titlebar(Some(&header));

	// Connect

	pulse_shr.borrow_mut().connect();
	pulse_shr.borrow_mut().subscribe();

	// Add Meters & Elements

	let meters_shr = Shared::new(Meters::new());

	let output = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	output.pack_start(&meters_shr.borrow().sink.widget, false, false, 0);
	output.set_border_width(4);

	let output_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	output_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	output_scroller.get_style_context().add_class("bordered");
	output.pack_start(&output_scroller, true, true, 0);
	output_scroller.add(&meters_shr.borrow().sink_inputs_box);

	let input = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	input.pack_start(&meters_shr.borrow().source.widget, false, false, 0);
	input.set_border_width(4);

	let input_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	input_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	input_scroller.get_style_context().add_class("bordered");
	input.pack_start(&input_scroller, true, true, 0);
	input_scroller.add(&meters_shr.borrow().source_outputs_box);

	let mut system_meter = StreamMeter::new();
	system_meter.set_name_and_icon("System Sounds", "multimedia-volume-control");
	system_meter.set_volume(65535);
	meters_shr.borrow().sink_inputs_box.pack_start(&system_meter.widget, false, false, 0);

	glib::timeout_add_local(1000 / 30, move || {
		update(&pulse_shr, &meters_shr);
		glib::Continue(true)
	});

	stack.add_titled(&output, "output", "Output");
	stack.add_titled(&input, "input", "Input");

	window.add(&stack);

	// window.add(&notebook.widget);

	window.show_all();

	// show_about();
}

fn update(pulse_shr: &Shared<PulseController>, meters_shr: &Shared<Meters>) {

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

			let meter = meters.sink_inputs.entry(*index).or_insert({
				let s = StreamMeter::new();
				let index: u32 = *index;

				let pulse = pulse_shr.clone();
				s.widgets.scale.connect_change_value(move |_, _, value| {
					pulse.borrow_mut().set_sink_input_volume(index, value as u32);
					gtk::Inhibit(false)
				});

				let pulse = pulse_shr.clone();
				s.widgets.status.connect_clicked(move |status| {
					pulse.borrow_mut().set_sink_input_muted(index,
						!status.get_style_context().has_class("muted"));
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

fn show_about() {
	let about = gtk::AboutDialog::new();
	about.set_logo_icon_name(Some("multimedia-volume-control"));
	about.set_program_name("VMix");
	about.set_version(Some("0.0.1-alpha"));
	about.set_comments(Some("Modern Volume Mixer for PulseAudio."));
	about.set_website(Some("https://www.aurailus.com"));
	// about.set_website_label(Some("Aurailus.com"));
	about.set_copyright(Some("© 2021 Auri Collings"));
	about.set_license_type(gtk::License::Gpl30);
	about.add_credit_section("Created by", &[ "Auri Collings" ]);
	about.add_credit_section("libpulse-binding by", &[ "Lyndon Brown" ]);

	about.connect_response(|about, _| about.close());
	about.run();
}

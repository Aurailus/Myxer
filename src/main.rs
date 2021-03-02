use std::collections::HashMap;

use gtk::prelude::*;
use gio::prelude::*;

mod meter;
mod about;
mod shared;
mod pulse_controller;

use meter::Meter;
use shared::Shared;
use pulse_controller::*;

struct Meters {
	pub sink: Meter,
	pub sink_inputs: HashMap<u32, Meter>,
	pub sink_inputs_box: gtk::Box,
	
	pub source: Meter,
	pub source_outputs: HashMap<u32, Meter>,
	pub source_outputs_box: gtk::Box,

	pub show_visualizers: bool
}

impl Meters {
	pub fn new() -> Self {
		let sink = Meter::new();
		sink.widget.get_style_context().add_class("outer");
		sink.widget.get_style_context().add_class("bordered");

		let source = Meter::new();
		source.widget.get_style_context().add_class("outer");
		source.widget.get_style_context().add_class("bordered");

		Meters {
			sink, source,
			show_visualizers: true,
			sink_inputs: HashMap::new(),
			sink_inputs_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
			source_outputs: HashMap::new(),
			source_outputs_box: gtk::Box::new(gtk::Orientation::Horizontal, 0)
		}
	}

	fn toggle_visualizers(&mut self) -> bool {
		self.show_visualizers = !self.show_visualizers;
		if self.show_visualizers { return true; }

		// self.sink.set_visualizer(None);
		// self.source.set_visualizer(None);
		// for (_, input) in self.sink_inputs.iter_mut() { input.set_visualizer(None); }
		// for (_, output) in self.source_outputs.iter_mut() { output.set_visualizer(None); }

		false
	}
}

fn main() {
	let pulse_shr = Shared::new(PulseController::new());

	let app = gtk::Application::new(Some("com.aurailus.myxer"), Default::default())
		.expect("Failed to initialize GTK application.");
		
	let pulse = pulse_shr.clone();
	app.connect_activate(move |app| activate(app, pulse.clone()));
	app.run(&[]);

	pulse_shr.borrow_mut().cleanup();
}

fn activate(app: &gtk::Application, pulse_shr: Shared<PulseController>) {
	let window = gtk::ApplicationWindow::new(app);
	let header = gtk::HeaderBar::new();
	let stack = gtk::Stack::new();
	
	// Window Config & Header Bar
	{
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

		window.set_geometry_hints::<gtk::ApplicationWindow>(None, Some(&geom), gdk::WindowHints::MIN_SIZE | gdk::WindowHints::MAX_SIZE);

		let style = include_str!("./style.css");
		let provider = gtk::CssProvider::new();
		provider.load_from_data(style.as_bytes()).expect("Failed to load CSS.");
		gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK css provider."),
			&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

		let stack_switcher = gtk::StackSwitcher::new();
		stack_switcher.set_stack(Some(&stack));

		header.set_show_close_button(true);
		header.set_custom_title(Some(&stack_switcher));

		let title = gtk::Label::new(Some("Volume Mixer"));
		title.get_style_context().add_class("title");
		header.pack_start(&title);
		header.set_decoration_layout(Some("icon:minimize,close"));
		
		window.set_titlebar(Some(&header));
	}

	// Preferences Button & Popup Menu
	{
		let prefs_button = gtk::Button::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::SmallToolbar);
		prefs_button.get_style_context().add_class("titlebutton");
		prefs_button.set_widget_name("preferences");
		prefs_button.set_can_focus(false);
		header.pack_end(&prefs_button);

		let prefs = gtk::PopoverMenu::new();
		prefs.set_pointing_to(&gtk::Rectangle { x: 12, y: 32, width: 2, height: 2 });
		prefs.set_relative_to(Some(&prefs_button));
		prefs.set_border_width(8);

		let prefs_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
		prefs.add(&prefs_box);

		let show_visualizers = gtk::ModelButton::new();
		show_visualizers.set_property_text(Some("Show Visualizers"));
		show_visualizers.set_action_name(Some("app.show_visualizers"));
		prefs_box.add(&show_visualizers);

		let split_channels = gtk::ModelButton::new();
		split_channels.set_property_text(Some("Split Channels"));
		split_channels.set_action_name(Some("app.split_channels"));
		prefs_box.add(&split_channels);

		prefs_box.pack_start(&gtk::Separator::new(gtk::Orientation::Horizontal), false, false, 4);

		let help = gtk::ModelButton::new();
		help.set_property_text(Some("Help"));
		help.set_action_name(Some("app.help"));
		prefs_box.add(&help);

		let about = gtk::ModelButton::new();
		about.set_property_text(Some("About Myxer"));
		about.set_action_name(Some("app.about"));
		prefs_box.add(&about);

		prefs_box.show_all();
		prefs_button.connect_clicked(move |_| prefs.popup());
	}

	// Connect Pulse

	pulse_shr.borrow_mut().connect();
	pulse_shr.borrow_mut().subscribe();

	let meters_shr = Shared::new(Meters::new());

	// Window Contents
	{
		let output = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		{
			let sink_meter = &mut meters_shr.borrow_mut().sink;
			// sink_meter.connect(StreamType::Sink, 0, &pulse_shr);
			// sink_meter.set_options(Some(&[ "Hello", "World", "This", "Is", "Here" ]), |_|());
		}

		output.pack_start(&meters_shr.borrow().sink.widget, false, false, 0);
		output.set_border_width(4);

		let output_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
		output_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
		output_scroller.get_style_context().add_class("bordered");
		output.pack_start(&output_scroller, true, true, 0);
		output_scroller.add(&meters_shr.borrow().sink_inputs_box);

		let input = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		let source_meter = &meters_shr.borrow().source;
		// source_meter.connect(StreamType::Source, 0, &pulse_shr);

		input.pack_start(&source_meter.widget, false, false, 0);
		input.set_border_width(4);

		let input_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
		input_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
		input_scroller.get_style_context().add_class("bordered");
		input.pack_start(&input_scroller, true, true, 0);
		input_scroller.add(&meters_shr.borrow().source_outputs_box);

		// let mut system_meter = OutputMeter::new();
		// system_meter.set_name_and_icon("System Sounds", "multimedia-volume-control");
		// system_meter.set_volume_and_muted(65535, false);
		// meters_shr.borrow().sink_inputs_box.pack_start(&system_meter.widget, false, false, 0);

		stack.add_titled(&output, "output", "Output");
		stack.add_titled(&input, "input", "Input");

		window.add(&stack);
		window.show_all();
	}

	// Actions
	{
		let actions = gio::SimpleActionGroup::new();
		window.insert_action_group("app", Some(&actions));

		let about = gio::SimpleAction::new("about", None);
		about.connect_activate(|_, _| about::about());
		actions.add_action(&about);

		let split_channels = gio::SimpleAction::new_stateful("split_channels", glib::VariantTy::new("bool").ok(), &false.to_variant());
		// split_channels.connect_activate(|_, _| show_about());
		actions.add_action(&split_channels);

		let meters_shr = meters_shr.clone();
		let show_visualizers = gio::SimpleAction::new_stateful("show_visualizers", glib::VariantTy::new("bool").ok(), &true.to_variant());
		show_visualizers.connect_activate(move |s, _| s.set_state(&meters_shr.borrow_mut().toggle_visualizers().to_variant()));
		actions.add_action(&show_visualizers);
	}

	// Begin Update Loop
	glib::timeout_add_local(1000 / 30, move || {
		update(&pulse_shr, &meters_shr);
		glib::Continue(true)
	});
}

fn update(pulse_shr: &Shared<PulseController>, meters_shr: &Shared<Meters>) {
	if pulse_shr.borrow_mut().update() {
		let pulse = pulse_shr.borrow();
		let mut meters = meters_shr.borrow_mut();
		let show = meters.show_visualizers;

		if let Some(sink_pair) = pulse.sinks.iter().next() {
			let sink = sink_pair.1;
			meters.sink.set_data(&sink.data);
			if show { meters.sink.set_peak(Some(sink.peak)); }
		}

		for (index, input) in pulse.sink_inputs.iter() {
			let sink_inputs_box = meters.sink_inputs_box.clone();

			let meter = meters.sink_inputs.entry(*index).or_insert_with(|| Meter::with_connection(&input.data, &pulse_shr));
			if meter.widget.get_parent().is_none() { sink_inputs_box.pack_start(&meter.widget, false, false, 0); }
			meter.set_data(&input.data);
			if show { meter.set_peak(Some(input.peak)); }
		}

		let sink_inputs_box = meters.sink_inputs_box.clone();
		meters.sink_inputs.retain(|index, meter| {
			let keep = pulse.sink_inputs.contains_key(index);
			if !keep { sink_inputs_box.remove(&meter.widget); }
			keep
		});

		if let Some(source_pair) = pulse.sources.iter().next() {
			let source = source_pair.1;
			meters.source.set_data(&source.data);
			if show { meters.source.set_peak(Some(source.peak)); }
		}

		for (index, output) in pulse.source_outputs.iter() {
			let source_outputs_box = meters.source_outputs_box.clone();
			
			let meter = meters.source_outputs.entry(*index).or_insert_with(|| Meter::with_connection(&output.data, &pulse_shr));
			if meter.widget.get_parent().is_none() { source_outputs_box.pack_start(&meter.widget, false, false, 0); }
			meter.set_data(&output.data);
			if show { meter.set_peak(Some(output.peak)); }
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

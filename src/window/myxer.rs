/*!
 * Contains he main application window, and associated data structs.
 */

use std::collections::HashMap;

use gtk::prelude::*;
use gio::prelude::*;

use super::style;
use crate::pulse::Pulse;
use crate::shared::Shared;
use super::{ about, Profiles };
use crate::meter::{ Meter, SinkMeter, SourceMeter, StreamMeter };


/**
 * Stores meter widgets and options.
 */

pub struct Meters {
	pub sink: SinkMeter,
	pub sink_box: gtk::Box,
	pub sink_inputs: HashMap<u32, StreamMeter>,
	pub sink_inputs_box: gtk::Box,

	pub source: SourceMeter,
	pub source_box: gtk::Box,
	pub source_outputs: HashMap<u32, StreamMeter>,
	pub source_outputs_box: gtk::Box,

	pub show_visualizers: bool,
	pub separate_channels: bool,
	pub remember_position: bool,

	pub window_position: (i32, i32),

	pub _config_path: std::path::PathBuf
}

impl Meters {

	/**
	 * Creates the Struct, and some base widgets,
	 * including the Sink and Source meters.
	 *
	 * * `pulse` - The Pulse instance used by the app.
	 */

	pub fn new(pulse: &Shared<Pulse>) -> Self {
		let sink = SinkMeter::new(pulse.clone());

		let sink_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		sink_box.get_style_context().add_class("pad_side");
		sink_box.add(&sink.widget);

		let source = SourceMeter::new(pulse.clone());

		let source_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		source_box.get_style_context().add_class("pad_side");
		source_box.add(&source.widget);

		let sink_inputs_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		sink_inputs_box.get_style_context().add_class("pad_side");

		let source_outputs_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		source_outputs_box.get_style_context().add_class("pad_side");

		// get config path
		let home_dir = std::env::var_os("HOME").ok_or("no home directory").unwrap();
		let mut config_path = std::path::PathBuf::new();
		config_path.push(&home_dir);
		config_path.push(".config");
		config_path.push("myxer");
		config_path.push("myxer.conf");

		// create the Meters struct
		let mut meters = Meters {
			sink, source,
			sink_box, source_box,
			sink_inputs_box, source_outputs_box,
			sink_inputs: HashMap::new(),
			source_outputs: HashMap::new(),
			show_visualizers: true,
			separate_channels: false,
			remember_position: false,

			window_position: (0, 0),

			_config_path: config_path
		};

		// load config
		let _ = meters.load_config();

		meters
	}


	/*
	 * Saves the current settings to the configuration file.
	 */

	pub fn save_config(&self) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
		let config_path = self._config_path.clone();

		let config_content = format!(
			"show_visualizers={}\nseparate_channels={}\nremember_position={}\nwindow_position_x={}\nwindow_position_y={}\n",
			self.show_visualizers, self.separate_channels, self.remember_position, self.window_position.0, self.window_position.1
		);

		let dir_name = std::path::Path::new(&config_path).parent().ok_or("incorrect directory")?;
		std::fs::create_dir_all(dir_name)?;
		std::fs::write(&config_path, config_content)?;

		Ok(config_path)
	}


	/**
	 * Loads the settings from the configuration file.
	 */

	pub fn load_config(&mut self) -> Result<(std::ffi::OsString, String), Box<dyn std::error::Error>> {
		let config_path = self._config_path.clone();

		let config_content = std::fs::read_to_string(&config_path)?;

		for line in config_content.lines() {
			let mut parts = line.split('=');
			let key = parts.next().ok_or("no key")?;
			let value = parts.next().ok_or("no value")?;

			match key {
				"show_visualizers" => self.show_visualizers = value.parse().unwrap_or(self.show_visualizers),
				"separate_channels" => self.separate_channels = value.parse().unwrap_or(self.separate_channels),
				"remember_position" => self.remember_position = value.parse().unwrap_or(self.remember_position),
				"window_position_x" => self.window_position.0 = value.parse().unwrap_or(self.window_position.0),
				"window_position_y" => self.window_position.1 = value.parse().unwrap_or(self.window_position.1),
				_ => {}
			}
		}

		Ok((config_path.file_name().unwrap().to_os_string(), config_content))
   }


	/**
	 * Toggles the show visualizers setting, and returns its current state.
	 */

	fn toggle_visualizers(&mut self) -> bool {
		self.show_visualizers = !self.show_visualizers;
		let _ = self.save_config();
		self.show_visualizers
	}


	/**
	 * Toggles the separate channels setting, and returns its current state.
	 */

	fn toggle_separate_channels(&mut self) -> bool {
		self.separate_channels = !self.separate_channels;
		let _ = self.save_config();
		self.separate_channels
	}


	/**
	 * Toggles the remember position setting, and returns its current state.
	 */

	fn toggle_remember_position(&mut self) -> bool {
		self.remember_position = !self.remember_position;
		let _ = self.save_config();
		self.remember_position
	}
}


/**
 * The main Myxer application window,
 * Displays meters for each sink, source, sink input, and source output.
 */

pub struct Myxer {
	window: gtk::ApplicationWindow,

	pulse: Shared<Pulse>,
	meters: Shared<Meters>,

	profiles: Shared<Option<Profiles>>
}

impl Myxer {

	/**
	 * Initializes the main window.
	 *
	 * * `app` - The GTK application.
	 * * `pulse` - The Pulse store instance.
	 */

	pub fn new(app: &gtk::Application, pulse: &Shared<Pulse>) -> Self {
		let window = gtk::ApplicationWindow::new(app);
		let header = gtk::HeaderBar::new();
		let stack = gtk::Stack::new();
		let meters = Shared::new(Meters::new(pulse));

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

			window.set_type_hint(gdk::WindowTypeHint::Dialog);
			window.set_geometry_hints::<gtk::ApplicationWindow>(None, Some(&geom), gdk::WindowHints::MIN_SIZE | gdk::WindowHints::MAX_SIZE);
			window.get_style_context().add_class("Myxer");
			style::style(&window);

			let stack_switcher = gtk::StackSwitcher::new();
			stack_switcher.set_stack(Some(&stack));

			header.set_show_close_button(true);
			header.set_custom_title(Some(&stack_switcher));

			let title_vert = gtk::Box::new(gtk::Orientation::Vertical, 0);
			header.pack_start(&title_vert);

			let title_hor = gtk::Box::new(gtk::Orientation::Horizontal, 0);
			title_vert.pack_start(&title_hor, true, true, 0);

			let icon = gtk::Image::from_icon_name(Some("multimedia-volume-control"), gtk::IconSize::Button);
			title_hor.pack_start(&icon, true, true, 3);
			let title = gtk::Label::new(Some("Volume Mixer"));
			title.get_style_context().add_class("title");
			title_hor.pack_start(&title, true, true, 0);

			window.set_titlebar(Some(&header));

			if meters.borrow().remember_position {
				window.move_(meters.borrow().window_position.0, meters.borrow().window_position.1);
			}
		}

		{
			let prefs_button = gtk::Button::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::SmallToolbar);
			prefs_button.get_style_context().add_class("flat");
			prefs_button.set_widget_name("preferences");
			prefs_button.set_can_focus(false);
			header.pack_end(&prefs_button);

			let prefs = gtk::PopoverMenu::new();
			prefs.set_pointing_to(&gtk::Rectangle { x: 12, y: 32, width: 2, height: 2 });
			prefs.set_relative_to(Some(&prefs_button));
			prefs.set_border_width(6);

			let prefs_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
			prefs.add(&prefs_box);

			let show_visualizers = gtk::ModelButton::new();
			show_visualizers.set_property_text(Some("Visualize Peaks"));
			show_visualizers.set_action_name(Some("app.show_visualizers"));
			prefs_box.add(&show_visualizers);

			let split_channels = gtk::ModelButton::new();
			split_channels.set_property_text(Some("Split Channels"));
			split_channels.set_action_name(Some("app.split_channels"));
			prefs_box.add(&split_channels);

			let remember_position = gtk::ModelButton::new();
			remember_position.set_property_text(Some("Remember Position"));
			remember_position.set_action_name(Some("app.remember_position"));
			prefs_box.add(&remember_position);

			let card_profiles = gtk::ModelButton::new();
			card_profiles.set_property_text(Some("Card Profiles..."));
			card_profiles.set_action_name(Some("app.card_profiles"));
			prefs_box.add(&card_profiles);

			prefs_box.pack_start(&gtk::Separator::new(gtk::Orientation::Horizontal), false, false, 4);

			let about = gtk::ModelButton::new();
			about.set_property_text(Some("About Myxer"));
			about.set_action_name(Some("app.about"));
			prefs_box.add(&about);

			prefs_box.show_all();
			prefs_button.connect_clicked(move |_| prefs.popup());
		}

		pulse.borrow_mut().connect();

		{
			let output = gtk::Box::new(gtk::Orientation::Horizontal, 0);
			output.pack_start(&meters.borrow_mut().sink_box, false, false, 0);

			output.pack_start(&gtk::Separator::new(gtk::Orientation::Vertical), false, true, 0);

			let output_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
			output_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
			output_scroller.get_style_context().add_class("bordered");
			output.pack_start(&output_scroller, true, true, 0);
			output_scroller.add(&meters.borrow().sink_inputs_box);

			let input = gtk::Box::new(gtk::Orientation::Horizontal, 0);
			input.pack_start(&meters.borrow_mut().source_box, false, false, 0);

			input.pack_start(&gtk::Separator::new(gtk::Orientation::Vertical), false, true, 0);

			let input_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
			input_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
			input_scroller.get_style_context().add_class("bordered");
			input.pack_start(&input_scroller, true, true, 0);
			input_scroller.add(&meters.borrow().source_outputs_box);

			stack.add_titled(&output, "output", "Output");
			stack.add_titled(&input, "input", "Input");

			window.add(&stack);
			window.show_all();
		}

		let profiles = Shared::new(None);

		{
			let window = window.clone();

			let actions = gio::SimpleActionGroup::new();
			window.insert_action_group("app", Some(&actions));

			let about = gio::SimpleAction::new("about", None);
			about.connect_activate(|_, _| about::about());
			actions.add_action(&about);

			let card_profiles = gio::SimpleAction::new("card_profiles", None);
			let pulse = pulse.clone();
			let profiles = profiles.clone();
			card_profiles.connect_activate(move |_, _| {
				profiles.replace(Some(Profiles::new(&window, &pulse)));
			});
			actions.add_action(&card_profiles);

			let meters_clone = meters.clone();
			let split_channels = gio::SimpleAction::new_stateful("split_channels", glib::VariantTy::new("bool").ok(), &(meters_clone.borrow().separate_channels).to_variant());
			split_channels.connect_activate(move |s, _| s.set_state(&meters_clone.borrow_mut().toggle_separate_channels().to_variant()));
			actions.add_action(&split_channels);

			let meters_clone = meters.clone();
			let remember_position = gio::SimpleAction::new_stateful("remember_position", glib::VariantTy::new("bool").ok(), &(meters_clone.borrow().remember_position).to_variant());
			remember_position.connect_activate(move |s, _| s.set_state(&meters_clone.borrow_mut().toggle_remember_position().to_variant()));
			actions.add_action(&remember_position);

			let meters_clone = meters.clone();
			let show_visualizers = gio::SimpleAction::new_stateful("show_visualizers", glib::VariantTy::new("bool").ok(), &(meters_clone.borrow().show_visualizers).to_variant());
			show_visualizers.connect_activate(move |s, _| s.set_state(&meters_clone.borrow_mut().toggle_visualizers().to_variant()));
			actions.add_action(&show_visualizers);
		}

		Self {
			window,
			pulse: pulse.clone(),
			meters,
			profiles
		}
	}


	/**
	 * Updates the app's widgets based on information stored in the Pulse instance.
	 * Kills the Card Profiles window if it has been requested.
	 */

	pub fn update(&mut self) {
		let mut kill = false;
		if let Some(profiles) = self.profiles.borrow_mut().as_mut() { kill = !profiles.update(); }
		if kill { self.profiles.replace(None); }

		if self.pulse.borrow_mut().update() {
			let mut pulse = self.pulse.borrow_mut();

			let mut meters = self.meters.borrow_mut();

			let offset = meters.sink.widget.get_allocation().height +
				meters.sink.widget.get_margin_bottom() - meters.sink_inputs_box.get_allocation().height;
			if offset != meters.sink.widget.get_margin_bottom() { meters.sink.widget.set_margin_bottom(offset) }

			let offset = meters.source.widget.get_allocation().height +
				meters.source.widget.get_margin_bottom() - meters.source_outputs_box.get_allocation().height;
			if offset != meters.source.widget.get_margin_bottom() { meters.source.widget.set_margin_bottom(offset) }


			let show = meters.show_visualizers;
			let separate = meters.separate_channels;


			if let Some(sink) = pulse.sinks.get(&pulse.active_sink) {
				meters.sink.set_data(&sink.data);

				// refresh the peaks if they have changed OR if the split channels setting has changed
				let peak = if show { Some(sink.peak) } else { None };
				let refresh_peaks = (meters.sink.peak != peak) || (meters.sink.split != separate && show);

				meters.sink.split_channels(separate);

				if refresh_peaks {
					meters.sink.set_peak(peak);
				}
			}

			const DECREASE: u32 = 2000;
			const REPETITIONS: u32 = 3;

			for (index, input) in &mut pulse.sink_inputs {
				let sink_inputs_box = meters.sink_inputs_box.clone();

				let meter = meters.sink_inputs.entry(*index).or_insert_with(|| StreamMeter::new(self.pulse.clone()));
				if meter.widget.get_parent().is_none() { sink_inputs_box.pack_start(&meter.widget, false, false, 0); }

				// gradually decrease the peak value if it is not changing
				if input.peak != 0 && Some(input.peak) == meter.peak {
					if input.repetitions < REPETITIONS { input.repetitions += 1; }
					else {
						if input.peak < DECREASE { input.peak = 0; }
						else { input.peak -= DECREASE; }
					}
				}
				else { input.repetitions = 0; }

				// refresh the peaks if they have changed OR if the split channels setting has changed
				let peak = if show { Some(input.peak) } else { None };
				let refresh_peaks = (meter.peak != peak) || (meter.split != separate && show);

				meter.set_data(&input.data);
				meter.split_channels(separate);

				if refresh_peaks {
					meter.set_peak(peak);
				}
			}

			let sink_inputs_box = meters.sink_inputs_box.clone();
			meters.sink_inputs.retain(|index, meter| {
				let keep = pulse.sink_inputs.contains_key(index);
				if !keep { sink_inputs_box.remove(&meter.widget); }
				keep
			});

			if let Some(source) = pulse.sources.get(&pulse.active_source) {
				meters.source.set_data(&source.data);
				meters.source.split_channels(separate);
				meters.source.set_peak(if show { Some(source.peak) } else { None });
			}

			for (index, output) in &pulse.source_outputs {
				let source_outputs_box = meters.source_outputs_box.clone();

				let meter = meters.source_outputs.entry(*index).or_insert_with(|| StreamMeter::new(self.pulse.clone()));
				if meter.widget.get_parent().is_none() { source_outputs_box.pack_start(&meter.widget, false, false, 0); }
				meter.set_data(&output.data);
				meter.split_channels(separate);
				meter.set_peak(if show { Some(output.peak) } else { None });
			}

			let source_outputs_box = meters.source_outputs_box.clone();
			meters.source_outputs.retain(|index, meter| {
				let keep = pulse.source_outputs.contains_key(index);
				if !keep { source_outputs_box.remove(&meter.widget); }
				keep
			});

			meters.sink_inputs_box.show_all();
			meters.source_outputs_box.show_all();


			if meters.window_position != self.window.get_position() && self.window.get_focus().is_some() {
				meters.window_position = self.window.get_position();
				let _ = meters.save_config();
			}
		}
	}
}

use std::collections::HashMap;

use gtk::prelude::*;
use gio::prelude::*;

use crate::about;
use crate::style;
use crate::meter::Meter;
use crate::shared::Shared;
use crate::profiles::Profiles;
use crate::pulse_controller::PulseController;

struct Container {
	profiles: Option<Profiles>
}

struct Meters {
	pub sink: Meter,
	pub active_sink: Option<u32>,
	pub sink_inputs: HashMap<u32, Meter>,
	pub sink_inputs_box: gtk::Box,
	
	pub source: Meter,
	pub active_source: Option<u32>,
	pub source_outputs: HashMap<u32, Meter>,
	pub source_outputs_box: gtk::Box,

	pub show_visualizers: bool,
	pub separate_channels: bool
}

impl Meters {
	pub fn new() -> Self {
		let sink = Meter::new(None);
		sink.widget.get_style_context().add_class("outer");
		sink.widget.get_style_context().add_class("bordered");

		let source = Meter::new(None);
		source.widget.get_style_context().add_class("outer");
		source.widget.get_style_context().add_class("bordered");

		Meters {
			sink, source,
			active_sink: None, active_source: None,
			show_visualizers: true,
			separate_channels: false,
			sink_inputs: HashMap::new(),
			sink_inputs_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
			source_outputs: HashMap::new(),
			source_outputs_box: gtk::Box::new(gtk::Orientation::Horizontal, 0)
		}
	}

	fn toggle_visualizers(&mut self) -> bool {
		self.show_visualizers = !self.show_visualizers;
		self.show_visualizers
	}

	fn toggle_separate_channels(&mut self) -> bool {
		self.separate_channels = !self.separate_channels;
		self.separate_channels
	}

	fn set_active_source(&mut self, ind: u32) {
		self.source.set_connection(None);
		self.active_source = Some(ind);
	}

	fn set_active_sink(&mut self, ind: u32) {
		self.sink.set_connection(None);
		self.active_sink = Some(ind);
	}
}

pub struct Myxer {
	pulse: Shared<PulseController>,
	meters: Shared<Meters>,

	profiles: Shared<Container>
}

impl Myxer {
	pub fn new(app: &gtk::Application, pulse: &Shared<PulseController>) -> Self {
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
			style::style(&window);

			let stack_switcher = gtk::StackSwitcher::new();
			stack_switcher.set_stack(Some(&stack));

			header.set_show_close_button(true);
			header.set_custom_title(Some(&stack_switcher));

			let title_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
			let title = gtk::Label::new(Some("Volume Mixer"));
			title.get_style_context().add_class("title");
			title_box.pack_start(&title, true, true, 0);
			header.pack_start(&title_box);
			header.set_decoration_layout(Some("icon:minimize,close"));
			
			window.set_titlebar(Some(&header));
		}

		// Preferences Button & Popup Menu
		{
			let prefs_button = gtk::Button::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::SmallToolbar);
			prefs_button.get_style_context().add_class("flat");
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
			show_visualizers.set_property_text(Some("Visualize Peaks"));
			show_visualizers.set_action_name(Some("app.show_visualizers"));
			prefs_box.add(&show_visualizers);

			let split_channels = gtk::ModelButton::new();
			split_channels.set_property_text(Some("Split Channels"));
			split_channels.set_action_name(Some("app.split_channels"));
			prefs_box.add(&split_channels);

			let card_profiles = gtk::ModelButton::new();
			card_profiles.set_property_text(Some("Card Profiles..."));
			card_profiles.set_action_name(Some("app.card_profiles"));
			prefs_box.add(&card_profiles);

			prefs_box.pack_start(&gtk::Separator::new(gtk::Orientation::Horizontal), false, false, 4);

			// let help = gtk::ModelButton::new();
			// help.set_property_text(Some("Help"));
			// help.set_action_name(Some("app.help"));
			// prefs_box.add(&help);

			let about = gtk::ModelButton::new();
			about.set_property_text(Some("About Myxer"));
			about.set_action_name(Some("app.about"));
			prefs_box.add(&about);

			prefs_box.show_all();
			prefs_button.connect_clicked(move |_| prefs.popup());
		}

		// Connect Pulse

		pulse.borrow_mut().connect();

		let meters = Shared::new(Meters::new());

		// Window Contents
		{
			let output = gtk::Box::new(gtk::Orientation::Horizontal, 0);
			{
				let pulse_clone = pulse.clone();
				let meters_clone = meters.clone();
				let sink_meter = &mut meters.borrow_mut().sink;
				sink_meter.connect_label_clicked(move |_| {
					let menu = gtk::Menu::new();
					let pulse = pulse_clone.borrow();

					let mut last: Option<gtk::RadioMenuItem> = None;
					for (i, v) in pulse.sinks.iter() {
						let button = gtk::RadioMenuItem::with_label(v.data.description.as_str());
						if let Some(last) = last { button.join_group(Some(&last)) }
						last = Some(button.clone());
						menu.add(&button);
						
						if let Some(a) = meters_clone.borrow().active_sink {
							if a == *i { button.set_active(true) }
						}
						
						let i = *i;
						let meters = meters_clone.clone();
						button.connect_toggled(move |c| {
							if !c.get_active() { return }
							meters.borrow_mut().set_active_sink(i);
						});
					}

					menu.add(&gtk::SeparatorMenuItem::new());

					let name = meters_clone.borrow().sink.get_name().to_owned();
					let button = gtk::CheckMenuItem::with_label("Default Output Device");
					button.set_active(name == pulse.default_sink);
					button.set_sensitive(!button.get_active());
					let pulse = pulse_clone.clone();
					button.connect_toggled(move |_| pulse.borrow().set_default_sink(&name));
					menu.add(&button);

					menu.show_all();
					menu.popup_easy(0, 0);
				});

				output.pack_start(&sink_meter.widget, false, false, 0);
				output.set_border_width(4);
			}


			let output_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
			output_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
			output_scroller.get_style_context().add_class("bordered");
			output.pack_start(&output_scroller, true, true, 0);
			output_scroller.add(&meters.borrow().sink_inputs_box);

			let input = gtk::Box::new(gtk::Orientation::Horizontal, 0);
			{
				let pulse_clone = pulse.clone();
				let meters_clone = meters.clone();
				let source_meter = &mut meters.borrow_mut().source;
				source_meter.connect_label_clicked(move |_| {
					let menu = gtk::Menu::new();
					let pulse = pulse_clone.borrow();

					let mut last: Option<gtk::RadioMenuItem> = None;
					for (i, v) in pulse.sources.iter() {
						let button = gtk::RadioMenuItem::with_label(v.data.description.as_str());
						if let Some(last) = last { button.join_group(Some(&last)) }
						last = Some(button.clone());
						menu.add(&button);
						
						if let Some(a) = meters_clone.borrow().active_source {
							if a == *i { button.set_active(true) }
						}

						let i = *i;
						let meters = meters_clone.clone();
						button.connect_toggled(move |c| {
							if !c.get_active() { return }
							meters.borrow_mut().set_active_source(i);
						});
					}

					menu.add(&gtk::SeparatorMenuItem::new());

					let name = meters_clone.borrow().source.get_name().to_owned();
					let button = gtk::CheckMenuItem::with_label("Default Input Device");
					button.set_active(name == pulse.default_source);
					button.set_sensitive(!button.get_active());
					let pulse = pulse_clone.clone();
					button.connect_toggled(move |_| pulse.borrow().set_default_source(&name));
					menu.add(&button);

					menu.show_all();
					menu.popup_easy(0, 0);
				});

				input.pack_start(&source_meter.widget, false, false, 0);
				input.set_border_width(4);
			}

			let input_scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
			input_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
			input_scroller.get_style_context().add_class("bordered");
			input.pack_start(&input_scroller, true, true, 0);
			input_scroller.add(&meters.borrow().source_outputs_box);

			// let mut system_meter = OutputMeter::new();
			// system_meter.set_name_and_icon("System Sounds", "multimedia-volume-control");
			// system_meter.set_volume_and_muted(65535, false);
			// meters.borrow().sink_inputs_box.pack_start(&system_meter.widget, false, false, 0);

			stack.add_titled(&output, "output", "Output");
			stack.add_titled(&input, "input", "Input");

			window.add(&stack);
			window.show_all();
		}

		let profiles = Shared::new(Container { profiles: None });

		// Actions
		{
			let actions = gio::SimpleActionGroup::new();
			window.insert_action_group("app", Some(&actions));

			let about = gio::SimpleAction::new("about", None);
			about.connect_activate(|_, _| about::about());
			actions.add_action(&about);

			let card_profiles = gio::SimpleAction::new("card_profiles", None);
			let pulse = pulse.clone();
			let window = window.clone();
			let profiles = profiles.clone();
			card_profiles.connect_activate(move |_, _| {
				profiles.borrow_mut().profiles = Some(Profiles::new(&window, &pulse));
			});
			actions.add_action(&card_profiles);

			let meters_clone = meters.clone();
			let split_channels = gio::SimpleAction::new_stateful("split_channels", glib::VariantTy::new("bool").ok(), &false.to_variant());
			split_channels.connect_activate(move |s, _| s.set_state(&meters_clone.borrow_mut().toggle_separate_channels().to_variant()));
			actions.add_action(&split_channels);

			let meters_clone = meters.clone();
			let show_visualizers = gio::SimpleAction::new_stateful("show_visualizers", glib::VariantTy::new("bool").ok(), &true.to_variant());
			show_visualizers.connect_activate(move |s, _| s.set_state(&meters_clone.borrow_mut().toggle_visualizers().to_variant()));
			actions.add_action(&show_visualizers);
		}

		Self {
			pulse: pulse.clone(),
			meters,
			profiles
		}
	}

	pub fn update(&mut self) {
		let mut kill = false;
		if let Some(profiles) = self.profiles.borrow_mut().profiles.as_mut() { kill = !profiles.update(); }
		if kill { self.profiles.borrow_mut().profiles = None; }

		if self.pulse.borrow_mut().update() {
			let pulse = self.pulse.borrow();
			let mut meters = self.meters.borrow_mut();
			let show = meters.show_visualizers;
			let separate = meters.separate_channels;

			if meters.active_sink.is_none() {
				for (index, sink) in pulse.sinks.iter() {
					if sink.data.name == pulse.default_sink {
						meters.active_sink = Some(*index);
						break;
					}
				}
			}

			if let Some(active_sink) = meters.active_sink {
				if let Some(sink) = &pulse.sinks.get(&active_sink) {
					if !meters.sink.is_connected() { meters.sink.set_connection(Some(self.pulse.clone())); }
					meters.sink.set_data(&sink.data);
					meters.sink.set_separate_channels(separate);
					meters.sink.set_peak(if show { Some(sink.peak) } else { None });
				}
				else { meters.active_sink = None; }
			}

			for (index, input) in pulse.sink_inputs.iter() {
				let sink_inputs_box = meters.sink_inputs_box.clone();

				let meter = meters.sink_inputs.entry(*index).or_insert_with(|| Meter::new(Some(self.pulse.clone())));
				if meter.widget.get_parent().is_none() { sink_inputs_box.pack_start(&meter.widget, false, false, 0); }
				meter.set_data(&input.data);
				meter.set_separate_channels(separate);
				meter.set_peak(if show { Some(input.peak) } else { None });
			}

			let sink_inputs_box = meters.sink_inputs_box.clone();
			meters.sink_inputs.retain(|index, meter| {
				let keep = pulse.sink_inputs.contains_key(index);
				if !keep { sink_inputs_box.remove(&meter.widget); }
				keep
			});

			if meters.active_source.is_none() {
				for (index, source) in pulse.sources.iter() {
					if source.data.name == pulse.default_source {
						meters.active_source = Some(*index);
						break;
					}
				}
			}

			if let Some(active_source) = meters.active_source {
				if let Some(source) = &pulse.sources.get(&active_source) {
					if !meters.source.is_connected() { meters.source.set_connection(Some(self.pulse.clone())); }
					meters.source.set_data(&source.data);
					meters.source.set_separate_channels(separate);
					meters.source.set_peak(if show { Some(source.peak) } else { None });
				}
				else { meters.active_source = None; }
			}

			for (index, output) in pulse.source_outputs.iter() {
				let source_outputs_box = meters.source_outputs_box.clone();
				
				let meter = meters.source_outputs.entry(*index).or_insert_with(|| Meter::new(Some(self.pulse.clone())));
				if meter.widget.get_parent().is_none() { source_outputs_box.pack_start(&meter.widget, false, false, 0); }
				meter.set_data(&output.data);
				meter.set_separate_channels(separate);
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
		}
	}
}

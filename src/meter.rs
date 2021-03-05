use gtk;
use gtk::prelude::*;
use glib::translate::ToGlib;
use glib::translate::FromGlib;
use pulse::volume::{ ChannelVolumes, Volume };

use crate::shared::Shared;
use crate::pulse_controller::{ PulseController, StreamType };

const MAX_NATURAL_VOL: u32 = 65536;
const MAX_SCALE_VOL: u32 = (MAX_NATURAL_VOL as f64 * 1.5) as u32;
const SCALE_STEP: f64 = MAX_NATURAL_VOL as f64 / 20.0;

const INPUT_ICONS: [&str; 4] = [ "microphone-sensitivity-muted-symbolic", "microphone-sensitivity-low-symbolic",
	"microphone-sensitivity-medium-symbolic", "microphone-sensitivity-high-symbolic" ];

const OUTPUT_ICONS: [&str; 4] = [ "audio-volume-muted-symbolic", "audio-volume-low-symbolic",
	"audio-volume-medium-symbolic", "audio-volume-high-symbolic" ];

#[derive(Debug, Clone, Default)]
pub struct MeterData {
	pub t: StreamType,
	pub index: u32,

	pub name: String,
	pub icon: String,
	pub description: String,

	pub volume: ChannelVolumes,
	pub muted: bool,
}

struct MeterWidgets {
	root: gtk::Box,
	
	icon: gtk::Image,
	label: gtk::Label,
	select: gtk::Button,
	scale_box: gtk::Box,
	scales: Vec<gtk::Scale>,
	status: gtk::Button,
	status_icon: gtk::Image
}

fn build_scale() -> gtk::Scale {
	let scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, MAX_SCALE_VOL as f64, SCALE_STEP);

	scale.set_inverted(true);
	scale.set_draw_value(false);
	scale.set_sensitive(false);
	scale.set_increments(SCALE_STEP, SCALE_STEP);
	scale.set_restrict_to_fill_level(false);

	scale.add_mark(0.0, gtk::PositionType::Right, Some(""));
	scale.add_mark(MAX_SCALE_VOL as f64, gtk::PositionType::Right, Some(""));
	scale.add_mark(MAX_NATURAL_VOL as f64, gtk::PositionType::Right, Some(""));

	scale
}

fn build() -> MeterWidgets {
	let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
	root.set_widget_name("meter");

	root.set_orientation(gtk::Orientation::Vertical);
	root.set_hexpand(false);
	root.set_size_request(86, -1);

	let prefs = gtk::PopoverMenu::new();

	let top_evt = gtk::Button::new();
	top_evt.set_widget_name("top");
	top_evt.get_style_context().add_class("flat");
	let prefs_clone = prefs.clone();
	top_evt.connect_clicked(move |_| {
		println!("Icon clicked~");
		prefs_clone.popup();
		prefs_clone.show_all();
		prefs_clone.get_children().iter().next().unwrap().show_all();
	});

	prefs.set_relative_to(Some(&top_evt));
	prefs.set_border_width(8);

	let split_channels = gtk::ModelButton::new();
	split_channels.set_property_text(Some("Split Channels"));
	split_channels.set_action_name(Some("app.split_channels"));

	let submenu_open = gtk::ModelButton::new();
	submenu_open.set_property("menu-name", &"more").unwrap();
	submenu_open.set_property_text(Some("Set Output Device"));

	let menu = gtk::Box::new(gtk::Orientation::Vertical, 0);
	menu.add(&split_channels);
	menu.add(&submenu_open);
	prefs.add(&menu);

	let submenu = gtk::Box::new(gtk::Orientation::Vertical, 0);
	let back = gtk::ModelButton::new();
	back.set_property_text(Some("Set Output Device"));
	back.set_property("inverted", &true).unwrap();
	back.set_property("centered", &true).unwrap();
	back.set_property("menu-name", &"main").unwrap();
	// back.set_action_name(Some("app.card_profiles"));


	submenu.pack_start(&back, false, false, 0);
	submenu.pack_start(&gtk::SeparatorMenuItem::new(), false, false, 4);

	let d = gtk::ModelButton::new();
	d.set_property_text(Some("Headphones"));
	submenu.pack_start(&d, false, false, 0);
	let d = gtk::ModelButton::new();
	d.set_property_text(Some("Fucki"));
	submenu.pack_start(&d, false, false, 0);
	let d = gtk::ModelButton::new();
	d.set_property_text(Some("Dick"));
	submenu.pack_start(&d, false, false, 0);

	prefs.add(&submenu);
	prefs.set_child_submenu(&submenu, Some("more"));



	let top_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
	top_evt.add(&top_container);

	let icon = gtk::Image::from_icon_name(Some("audio-volume-muted-symbolic"), gtk::IconSize::Dnd);

	let label = gtk::Label::new(Some("Unknown"));
	label.set_widget_name("app_label");

	label.set_size_request(-1, 42);
	label.set_justify(gtk::Justification::Center);
	label.set_ellipsize(pango::EllipsizeMode::End);
	label.set_line_wrap_mode(pango::WrapMode::WordChar);
	label.set_max_width_chars(8);
	label.set_line_wrap(true);
	label.set_lines(2);

	top_container.pack_end(&label, false, true, 0);
	top_container.pack_end(&icon, false, false, 4);

	let select = gtk::Button::new();
	select.set_widget_name("app_select");
	select.get_style_context().add_class("flat");

	let scale_box_o = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	let scale_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	scale_box_o.pack_start(&scale_box, true, false, 0);
	let scales = Vec::new();

	let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

	let status_icon = gtk::Image::from_icon_name(Some("audio-volume-muted-symbolic"), gtk::IconSize::Button);
	
	let status = gtk::Button::new();
	status.set_widget_name("mute_toggle");
	status_box.pack_start(&status, true, false, 0);
	
	status.set_image(Some(&status_icon));
	status.set_always_show_image(true);
	status.get_style_context().add_class("flat");
	status.get_style_context().add_class("muted");

	root.pack_end(&status_box, false, false, 4);
	root.pack_end(&scale_box_o, true, true, 2);
	root.pack_start(&top_evt, false, false, 0);

	MeterWidgets {
		root,
		icon,
		label,
		select,
		scale_box,
		scales,
		status,
		status_icon
	}
}

pub struct Meter {
	pub widget: gtk::Box,
	widgets: MeterWidgets,
	separate: bool,

	data: MeterData,
	peak: Option<u32>,

	pulse: Option<Shared<PulseController>>,
	status_connect_id: Option<glib::signal::SignalHandlerId>,
}

impl Meter {
	pub fn new(pulse: Option<Shared<PulseController>>) -> Self {
		let widgets = build();
		Self {
			widget: widgets.root.clone(),
			widgets, data: MeterData::default(),
			pulse,
			separate: false,
			peak: Some(0),
			status_connect_id: None
		}
	}

	pub fn set_separate_channels(&mut self, separate: bool) {
		if self.separate == separate { return }
		self.separate = separate;
		self.reset_connection();
	}

	pub fn set_connection(&mut self, pulse: Option<Shared<PulseController>>) {
		self.pulse = pulse;
		self.reset_connection();
	}

	pub fn is_connected(&self) -> bool {
		self.pulse.is_some()
	}

	pub fn get_name(&self) -> &str {
		self.data.name.as_str()
	}

	fn reset_connection(&mut self) {
		self.disconnect();
		if self.pulse.is_none() { return; }

		let t = self.data.t;
		let index = self.data.index;

		let pulse = self.pulse.as_ref().unwrap().clone();
		self.status_connect_id = Some(self.widgets.status.connect_clicked(move |status| {
			pulse.borrow_mut().set_muted(t, index,
				!status.get_style_context().has_class("muted"));
		}));

		if self.separate {
			for _ in 0 .. self.data.volume.len() {
				let scale = build_scale();
				let pulse = self.pulse.as_ref().unwrap().clone();
				scale.connect_change_value(move |scale, _, val| {

					let parent = scale.get_parent().unwrap().downcast::<gtk::Box>().unwrap();
					let children = parent.get_children();
					
					let mut volumes = ChannelVolumes::default();
					
					// So, if you're wondering why rev() is necessary or why I set the len after or why this is horrible in general,
					// Check out libpulse_binding::volumes::ChannelVolumes::set, and you'll see ._.
					for (i, w) in children.iter().enumerate().rev() {
						let s = w.clone().downcast::<gtk::Scale>().unwrap();
						let value = if *scale == s { val } else { s.get_value() };
						let volume = Volume(value as u32);
						volumes.set(i as u8 + 1, volume);
					}

					volumes.set_len(children.len() as u8);

					pulse.borrow_mut().set_volume(t, index, volumes);
					gtk::Inhibit(false)
				});
				self.widgets.scale_box.pack_start(&scale, false, false, 0);
				self.widgets.scales.push(scale);
			}
		}
		else {
			let scale = build_scale();
			let channels = self.data.volume.len();
			let pulse = self.pulse.as_ref().unwrap().clone();
			scale.connect_change_value(move |_, _, value| {
				let mut volumes = ChannelVolumes::default();
				volumes.set_len(channels);
				volumes.set(channels, Volume(value as u32));
				pulse.borrow_mut().set_volume(t, index, volumes);
				gtk::Inhibit(false)
			});
			self.widgets.scale_box.pack_start(&scale, false, false, 0);
			self.widgets.scales.push(scale);
		}

		for (i, v) in self.data.volume.get().iter().enumerate() {
			if let Some(scale) = self.widgets.scales.get(i) {
				scale.set_sensitive(!self.data.muted);
				scale.set_value(v.0 as f64);
				if self.peak.is_some() { scale.get_style_context().add_class("visualizer") }
			}
		}

		self.widgets.scale_box.show_all();
	}

	fn disconnect(&mut self) {
		if self.status_connect_id.is_some() {
			self.widgets.status.disconnect(glib::signal::SignalHandlerId::from_glib(self.status_connect_id.as_ref().unwrap().to_glib()));
		}
		for s in self.widgets.scales.iter() { self.widgets.scale_box.remove(s); }
		self.widgets.scales.clear();
	}

	pub fn set_data(&mut self, data: &MeterData) {
		let volume_old = self.data.volume;
		let volume_changed = data.volume != volume_old;

		if self.pulse.is_some() && (data.t != self.data.t || data.index != self.data.index || data.volume.len() != self.data.volume.len()) {
			self.data.t = data.t;
			self.data.volume = data.volume;
			self.data.index = data.index;
			self.reset_connection();
		}

		if data.icon != self.data.icon {
			self.data.icon = data.icon.clone();
			self.widgets.icon.set_from_icon_name(Some(&self.data.icon.as_str()), gtk::IconSize::Dnd);
		}

		if data.name != self.data.name {
			self.data.name = data.name.clone();
		}

		if data.description != self.data.description {
			self.data.description = data.description.clone();
			self.widgets.label.set_label(self.data.description.as_str());
		}

		if volume_changed || data.muted != self.data.muted {
			self.data.volume = data.volume;
			self.data.muted = data.muted;

			for (i, v) in self.data.volume.get().iter().enumerate() {
				if let Some(scale) = self.widgets.scales.get(i) {
					scale.set_sensitive(!self.data.muted);
					scale.set_value(v.0 as f64);
				}
			}

			let status_vol = self.data.volume.max().0;

			let &icons = if self.data.t == StreamType::Sink || self.data.t == StreamType::SinkInput
				{ &OUTPUT_ICONS } else { &INPUT_ICONS };

			self.widgets.status_icon.set_from_icon_name(Some(icons[
				if self.data.muted { 0 } else if status_vol >= MAX_NATURAL_VOL { 3 }
				else if status_vol >= MAX_NATURAL_VOL / 2 { 2 } else { 1 }]), gtk::IconSize::Button);

			let mut vol_scaled = ((status_vol as f64) / MAX_NATURAL_VOL as f64 * 100.0).round() as u8;
			if vol_scaled > 150 { vol_scaled = 150 }

			let mut string = vol_scaled.to_string();
			string.push_str("%");
			self.widgets.status.set_label(string.as_str());

			let status_ctx = self.widgets.status.get_style_context();
			if self.data.muted { status_ctx.add_class("muted") }
			else { status_ctx.remove_class("muted") }
		}
	}

	pub fn set_peak(&mut self, peak: Option<u32>) {
		if self.peak != peak {
			self.peak = peak;

			if self.peak.is_some() {
				for (i, s) in self.widgets.scales.iter().enumerate() {
					let peak_scaled = self.peak.unwrap() as f64 * (self.data.volume.get()[i].0 as f64 / MAX_SCALE_VOL as f64);
					s.set_fill_level(peak_scaled as f64);
					s.set_show_fill_level(!self.data.muted && peak_scaled > 0.5);
					s.get_style_context().add_class("visualizer");
				}
			}
			else {
				for s in self.widgets.scales.iter() {
					s.set_show_fill_level(false);
					s.get_style_context().remove_class("visualizer");
				}
			}
		}
	}

	pub fn connect_label_clicked<F: Fn(&gtk::Button) + 'static>(&mut self, f: F) {
		self.widget.remove(&self.widgets.label);
		self.widget.remove(&self.widgets.icon);

		self.widgets.select.add(&self.widgets.label);
		self.widgets.select.connect_clicked(f);

		self.widget.pack_start(&self.widgets.icon, false, false, 4);
		self.widget.pack_start(&self.widgets.select, false, false, 0);
	}
}

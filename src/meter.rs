use gtk;
use gtk::prelude::*;

use crate::shared::Shared;
use crate::pulse_controller::{ PulseController, StreamType };

const MAX_NATURAL_VOL: u32 = 65536;
const MAX_SCALE_VOL: u32 = (MAX_NATURAL_VOL as f64 * 1.5) as u32;
const SCALE_STEP: f64 = MAX_NATURAL_VOL as f64 / 20.0;

const INPUT_ICONS: [&str; 4] = [ "microphone-sensitivity-muted-symbolic", "microphone-sensitivity-low-symbolic",
	"microphone-sensitivity-medium-symbolic", "microphone-sensitivity-high-symbolic" ];

const OUTPUT_ICONS: [&str; 4] = [ "audio-volume-muted-symbolic", "audio-volume-low-symbolic",
	"audio-volume-medium-symbolic", "audio-volume-high-symbolic" ];

#[derive(Debug)]
#[derive(Clone)]
#[derive(Default)]
pub struct MeterData {
	pub t: StreamType,
	pub index: u32,

	pub name: String,
	pub icon: Option<String>,

	pub volume: u32,
	pub muted: bool,
}

struct MeterWidgets {
	root: gtk::Box,
	
	icon: gtk::Image,
	label: gtk::Label,
	select: gtk::Button,
	popover_box: gtk::Box,
	scale: gtk::Scale,
	status: gtk::Button,
	status_icon: gtk::Image
}

fn build() -> MeterWidgets {
	let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
	root.set_widget_name("meter");

	root.set_orientation(gtk::Orientation::Vertical);
	root.set_hexpand(false);
	root.set_size_request(86, -1);

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

	let select = gtk::Button::new();
	select.set_widget_name("app_select");
	select.get_style_context().add_class("flat");

	let select_popover = gtk::Popover::new(Some(&select));
	select_popover.set_border_width(4);

	let select_popover_clone = select_popover.clone();
	select.connect_clicked(move |_| select_popover_clone.popup());

	let popover_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
	select_popover.add(&popover_box);

	let scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, MAX_SCALE_VOL as f64, SCALE_STEP);
	scale.get_style_context().add_class("visualizer");

	scale.set_inverted(true);
	scale.set_draw_value(false);
	scale.set_sensitive(false);
	scale.set_increments(SCALE_STEP, SCALE_STEP);

	scale.set_fill_level(0.0);
	scale.set_show_fill_level(true);
	scale.set_restrict_to_fill_level(false);

	scale.add_mark(0.0, gtk::PositionType::Right, Some(""));
	scale.add_mark(MAX_SCALE_VOL as f64, gtk::PositionType::Right, Some(""));
	scale.add_mark(MAX_NATURAL_VOL as f64, gtk::PositionType::Right, Some(""));

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
	root.pack_end(&scale, true, true, 2);
	root.pack_end(&label, false, true, 0);
	root.pack_end(&icon, false, false, 4);

	MeterWidgets {
		root,
		icon,
		label,
		select,
		popover_box,
		scale,
		status,
		status_icon
	}
}

pub struct Meter {
	pub widget: gtk::Box,
	widgets: MeterWidgets,

	data: MeterData,
	peak: Option<u32>
}

impl Meter {
	pub fn new() -> Self {
		let widgets = build();
		Self { widget: widgets.root.clone(), widgets, data: MeterData::default(), peak: Some(0) }
	}

	pub fn with_connection(data: &MeterData, pulse_shr: &Shared<PulseController>) -> Self {
		let mut s = Meter::new();
		s.data.t = data.t;
		s.data.index = data.index;
		s.connect(pulse_shr);
		s
	}

	pub fn connect(&self, pulse_shr: &Shared<PulseController>) {
		let pulse = pulse_shr.clone();
		let t = self.data.t;
		let index = self.data.index;

		self.widgets.scale.connect_change_value(move |_, _, value| {
			pulse.borrow_mut().set_volume(t, index, value as u32);
			gtk::Inhibit(false)
		});

		let pulse = pulse_shr.clone();
		self.widgets.status.connect_clicked(move |status| {
			pulse.borrow_mut().set_muted(t, index,
				!status.get_style_context().has_class("muted"));
		});
	}

	pub fn set_data(&mut self, data: &MeterData) {
		self.data.t = data.t;
		self.data.index = data.index;

		if data.icon != self.data.icon {
			self.data.icon = data.icon.clone();
			let icon = if self.data.icon.is_some() { self.data.icon.as_ref().unwrap().as_str() } else { &"audio-volume-muted-symbolic" };
			self.widgets.icon.set_from_icon_name(Some(&icon), gtk::IconSize::Dnd);
		}

		if data.name != self.data.name {
			self.data.name = data.name.clone();
			self.widgets.label.set_label(self.data.name.as_str());
		}

		if data.volume != self.data.volume || data.muted != self.data.muted {
			self.data.volume = data.volume;
			self.data.muted = data.muted;
			
			self.widgets.scale.set_sensitive(!self.data.muted);
			self.widgets.scale.set_value(self.data.volume as f64);

			let &icons = if self.data.t == StreamType::Sink || self.data.t == StreamType::SinkInput
				{ &OUTPUT_ICONS } else { &INPUT_ICONS };

			self.widgets.status_icon.set_from_icon_name(Some(icons[
				if self.data.muted { 0 } else if self.data.volume >= MAX_NATURAL_VOL { 3 }
				else if self.data.volume >= MAX_NATURAL_VOL / 2 { 2 } else { 1 }]), gtk::IconSize::Button);

			let mut vol_scaled = ((self.data.volume as f64) / MAX_NATURAL_VOL as f64 * 100.0).round() as u8;
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
				let peak_scaled = self.peak.unwrap() as f64 * (self.data.volume as f64 / MAX_SCALE_VOL as f64);
				self.widgets.scale.set_fill_level(peak_scaled as f64);
				self.widgets.scale.set_show_fill_level(!self.data.muted && peak_scaled > 0.5);
				self.widgets.scale.get_style_context().add_class("visualizer");
			}
			else {
				self.widgets.scale.set_show_fill_level(false);
				self.widgets.scale.get_style_context().remove_class("visualizer");
			}
		}
	}
}

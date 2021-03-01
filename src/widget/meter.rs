use gtk;
use gtk::prelude::*;

const MAX_NATURAL_VOL: u32 = 65536;
const MAX_SCALE_VOL: u32 = (MAX_NATURAL_VOL as f64 * 1.5) as u32;
const SCALE_STEP: f64 = MAX_NATURAL_VOL as f64 / 20.0;

pub struct Widgets {
	root: gtk::Box,
	icon: gtk::Image,
	label: gtk::Label,
	pub scale: gtk::Scale,
	pub status: gtk::Button,
	status_icon: gtk::Image
}

fn build_widget() -> Widgets {
	let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
	root.set_widget_name("meter");

	root.set_orientation(gtk::Orientation::Vertical);
	root.set_hexpand(false);
	root.set_size_request(86, -1);

	let icon = gtk::Image::from_icon_name(Some("audio-card"), gtk::IconSize::Dnd);
	root.pack_start(&icon, false, false, 4);

	let label = gtk::Label::new(Some("Audio Meter"));
	label.set_widget_name("app_label");
	root.pack_start(&label, false, true, 0);

	label.set_size_request(-1, 42);
	label.set_justify(gtk::Justification::Center);
	label.set_ellipsize(pango::EllipsizeMode::End);
	label.set_line_wrap_mode(pango::WrapMode::WordChar);
	label.set_max_width_chars(8);
	label.set_line_wrap(true);
	label.set_lines(2);

	let scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, MAX_SCALE_VOL as f64, SCALE_STEP);
	scale.get_style_context().add_class("visualizer");
	root.pack_start(&scale, true, true, 2);

	scale.set_inverted(true);
	scale.set_draw_value(false);
	scale.set_increments(SCALE_STEP, SCALE_STEP);

	scale.set_fill_level(0.0);
	scale.set_show_fill_level(true);
	scale.set_restrict_to_fill_level(false);

	scale.add_mark(0.0, gtk::PositionType::Right, Some(""));
	scale.add_mark(MAX_SCALE_VOL as f64, gtk::PositionType::Right, Some(""));
	scale.add_mark(MAX_NATURAL_VOL as f64, gtk::PositionType::Right, Some(""));

	let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	root.pack_start(&status_box, false, false, 4);

	let status_icon = gtk::Image::from_icon_name(Some("audio-volume-low-symbolic"), gtk::IconSize::Button);
	
	let status = gtk::Button::with_label("0%");
	status.set_widget_name("mute_toggle");
	status_box.pack_start(&status, true, false, 0);
	
	status.set_image(Some(&status_icon));
	status.set_always_show_image(true);
	status.get_style_context().add_class("flat");

	Widgets {
		root,
		scale,
		label, icon,
		status, status_icon
	}
}

pub trait Meter {
	fn new() -> Self;

	fn set_visualizer(&mut self, peak: Option<u32>);
	fn set_name_and_icon(&mut self, name: &str, icon_name: &str);
	fn set_volume_and_muted(&mut self, volume: u32, muted: bool);
}

pub struct OutputMeter {
	pub widget: gtk::Box,
	pub widgets: Widgets,
	
	volume: u32,
	muted: bool,

	peak: Option<u32>
}

impl Meter for OutputMeter {
	fn new() -> Self {
		let widgets = build_widget();
		Self {
			widget: widgets.root.clone(),
			widgets,
			volume: 0,
			muted: false,

			peak: Some(0)
		}
	}

	fn set_volume_and_muted(&mut self, volume: u32, muted: bool) {
		if volume == self.volume && muted == self.muted { return; }

		self.volume = volume;
		self.muted = muted;
		
		self.widgets.scale.set_sensitive(!self.muted);
		self.widgets.scale.set_value(self.volume as f64);

		self.widgets.status_icon.set_from_icon_name(Some(
			if self.muted { "audio-volume-muted-symbolic" }
			else if self.volume >= MAX_NATURAL_VOL { "audio-volume-high-symbolic" }
			else if self.volume >= MAX_NATURAL_VOL / 2 { "audio-volume-medium-symbolic" }
			else { "audio-volume-low-symbolic" }), gtk::IconSize::Button);

		let mut vol_scaled = ((self.volume as f64) / MAX_NATURAL_VOL as f64 * 100.0).round() as u8;
		if vol_scaled > 150 { vol_scaled = 150 }

		let mut string = vol_scaled.to_string();
		string.push_str("%");
		self.widgets.status.set_label(string.as_str());

		let status_ctx = self.widgets.status.get_style_context();
		if self.muted { status_ctx.add_class("muted") }
		else { status_ctx.remove_class("muted") }
	}

	fn set_visualizer(&mut self, peak: Option<u32>) {
		if self.peak == peak { return; }

		self.peak = peak;

		if self.peak.is_some() {
			let peak_scaled = self.peak.unwrap() as f64 * (self.volume as f64 / MAX_SCALE_VOL as f64);
			self.widgets.scale.set_fill_level(peak_scaled as f64);
			self.widgets.scale.set_show_fill_level(!self.muted && peak_scaled > 0.5);
			self.widgets.scale.get_style_context().add_class("visualizer");
		}
		else {
			self.widgets.scale.set_show_fill_level(false);
			self.widgets.scale.get_style_context().remove_class("visualizer");
		}
	}

	fn set_name_and_icon(&mut self, label: &str, icon: &str) {
		self.widgets.icon.set_from_icon_name(Some(icon), gtk::IconSize::Dnd);
		self.widgets.label.set_label(label);
	}
}

pub struct InputMeter {
	pub widget: gtk::Box,
	pub widgets: Widgets,
	
	volume: u32,
	muted: bool,

	peak: Option<u32>
}

impl Meter for InputMeter {
	fn new() -> Self {
		let widgets = build_widget();
		Self {
			widget: widgets.root.clone(),
			widgets,
			volume: 0,
			muted: false,

			peak: Some(0)
		}
	}

	fn set_volume_and_muted(&mut self, volume: u32, muted: bool) {
		if volume == self.volume && muted == self.muted { return; }

		self.volume = volume;
		self.muted = muted;
		
		self.widgets.scale.set_sensitive(!self.muted);
		self.widgets.scale.set_value(self.volume as f64);

		self.widgets.status_icon.set_from_icon_name(Some(
			if self.muted { "microphone-sensitivity-muted-symbolic" }
			else if self.volume >= MAX_NATURAL_VOL { "microphone-sensitivity-high-symbolic" }
			else if self.volume >= MAX_NATURAL_VOL / 2 { "microphone-sensitivity-medium-symbolic" }
			else { "microphone-sensitivity-low-symbolic" }), gtk::IconSize::Button);

		let mut vol_scaled = ((self.volume as f64) / MAX_NATURAL_VOL as f64 * 100.0).round() as u8;
		if vol_scaled > 150 { vol_scaled = 150 }

		let mut string = vol_scaled.to_string();
		string.push_str("%");
		self.widgets.status.set_label(string.as_str());

		let status_ctx = self.widgets.status.get_style_context();
		if self.muted { status_ctx.add_class("muted") }
		else { status_ctx.remove_class("muted") }
	}

	fn set_visualizer(&mut self, peak: Option<u32>) {
		if self.peak == peak { return; }

		self.peak = peak;

		if self.peak.is_some() {
			let peak_scaled = self.peak.unwrap() as f64 * (self.volume as f64 / MAX_SCALE_VOL as f64);
			self.widgets.scale.set_fill_level(peak_scaled as f64);
			self.widgets.scale.set_show_fill_level(!self.muted && peak_scaled > 0.5);
			self.widgets.scale.get_style_context().add_class("visualizer");
		}
		else {
			self.widgets.scale.set_show_fill_level(false);
			self.widgets.scale.get_style_context().remove_class("visualizer");
		}
	}

	fn set_name_and_icon(&mut self, label: &str, icon: &str) {
		self.widgets.icon.set_from_icon_name(Some(icon), gtk::IconSize::Dnd);
		self.widgets.label.set_label(label);
	}
}

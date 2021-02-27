use gtk;
use gtk::prelude::*;

const MAX_NATURAL_VOL: f64 = 65536.0;

struct Widgets {
	root: gtk::Box,
	icon: gtk::Image,
	label: gtk::Label,
	scale: gtk::Scale,
	status: gtk::Button,
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

	let scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 150.0, 5.0);
	root.pack_start(&scale, true, true, 2);

	scale.set_inverted(true);
	scale.set_draw_value(false);
	scale.set_increments(5.0, 5.0);

	scale.set_fill_level(0.0);
	scale.set_show_fill_level(true);
	scale.set_restrict_to_fill_level(false);

	scale.add_mark(0.0, gtk::PositionType::Right, Some(""));
	scale.add_mark(100.0, gtk::PositionType::Right, Some(""));
	scale.add_mark(150.0, gtk::PositionType::Right, Some(""));

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

	fn set_muted(&mut self, muted: bool);
	fn set_volume(&mut self, volume: u32);
	fn set_peak_volume(&mut self, peak: u32);
	fn set_name_and_icon(&mut self, name: &str, icon_name: &str);

		// let img_ref = image.clone();
		// mute.connect_clicked(move |mute| {
		// 	if mute.get_style_context().has_class("muted") {
		// 		mute.get_style_context().remove_class("muted");
		// 		img_ref.set_from_icon_name(Some("audio-volume-low-symbolic"), gtk::IconSize::Button);
		// 	}
		// 	else {
		// 		mute.get_style_context().add_class("muted");
		// 		img_ref.set_from_icon_name(Some("action-unavailable-symbolic"), gtk::IconSize::Button);
		// 	}
		// });

		// let mute_ref = mute.clone();
		// let mute_icon_ref = mute_icon.clone();
		// scale.connect_value_changed(move |scale| {
		// 	let value = scale.get_value().floor();

		// 	if value >= 100.0 {
		// 		mute_icon_ref.set_from_icon_name(Some("audio-volume-high-symbolic"), gtk::IconSize::Button);
		// 	}
		// 	else if value >= 10.0 {
		// 		mute_icon_ref.set_from_icon_name(Some("audio-volume-medium-symbolic"), gtk::IconSize::Button);
		// 	}
		// 	else {
		// 		mute_icon_ref.set_from_icon_name(Some("audio-volume-low-symbolic"), gtk::IconSize::Button);
		// 	}

		// 	let mut string = value.to_string();
		// 	string.push_str("%");
		// 	mute_ref.set_label(string.as_str());
		// 	scale.set_fill_level(scale.get_value() / 2.0);
		// });
		
		
		// self_.show_all();
}

pub struct SinkMeter {
	pub widget: gtk::Box,
	widgets: Widgets,
	volume: u32,
	peak: u32,
	muted: bool
}

impl SinkMeter {
	fn update_status(&mut self) {
		let mut vol_scaled = ((self.volume as f64) / MAX_NATURAL_VOL * 100.0).round() as u8;
		if vol_scaled > 150 { vol_scaled = 150 }

		let peak_scaled = (self.peak as f64 * vol_scaled as f64 / 150.0) as u8;
		// self.widgets.label.set_sensitive(!self.muted);

		// println!("{:?}", self.peak);

		self.widgets.scale.set_sensitive(!self.muted);
		self.widgets.scale.set_value(vol_scaled.into());
		self.widgets.scale.set_show_fill_level(!self.muted && peak_scaled > 0);
		self.widgets.scale.set_fill_level(peak_scaled.into());

		self.widgets.status_icon.set_from_icon_name(Some(
			if self.muted { "action-unavailable-symbolic" }
			else if vol_scaled >= 100 { "audio-volume-high-symbolic" }
			else if vol_scaled >= 10 { "audio-volume-medium-symbolic" }
			else { "audio-volume-low-symbolic" }), gtk::IconSize::Button);


		let mut string = vol_scaled.to_string();
		string.push_str("%");
		self.widgets.status.set_label(string.as_str());

		let status_ctx = self.widgets.status.get_style_context();
		if self.muted { status_ctx.add_class("muted") }
		else { status_ctx.remove_class("muted") }
	}
}

impl Meter for SinkMeter {
	fn new() -> Self {
		let widgets = build_widget();
		Self {
			widget: widgets.root.clone(),
			widgets,
			peak: 0,
			volume: 0,
			muted: false
		}
	}

	fn set_muted(&mut self, muted: bool) {
		self.muted = muted;
		self.update_status();
	}

	fn set_volume(&mut self, volume: u32) {
		self.volume = volume;
		self.update_status();
	}

	fn set_peak_volume(&mut self, peak: u32) {
		self.peak = peak;
		self.update_status();
	}

	fn set_name_and_icon(&mut self, label: &str, icon: &str) {
		self.widgets.icon.set_from_icon_name(Some(icon), gtk::IconSize::Dnd);
		self.widgets.label.set_label(label);
	}
}

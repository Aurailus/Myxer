use gtk;
use gtk::prelude::*;
use pulse::volume::{ Volume, ChannelVolumes };

use crate::shared::Shared;
use crate::pulse_controller::{ PulseController, StreamType };

// The maximum natural volume, i.e. 100%
pub const MAX_NATURAL_VOL: u32 = 65536;

// The maximum scale volume, i.e. 150%
pub const MAX_SCALE_VOL: u32 = (MAX_NATURAL_VOL as f64 * 1.5) as u32;

// The increment step of the scale, e.g. how far it moves when you press up & down.
pub const SCALE_STEP: f64 = MAX_NATURAL_VOL as f64 / 20.0;

// The icon names for the input meter statuses.
pub const INPUT_ICONS: [&str; 4] = [ "microphone-sensitivity-muted-symbolic", "microphone-sensitivity-low-symbolic",
	"microphone-sensitivity-medium-symbolic", "microphone-sensitivity-high-symbolic" ];

// The icon names for the output meter statuses.
pub const OUTPUT_ICONS: [&str; 4] = [ "audio-volume-muted-symbolic", "audio-volume-low-symbolic",
	"audio-volume-medium-symbolic", "audio-volume-high-symbolic" ];

// Contains properties controlling a meter's display.
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

// Trait for all meter types.
pub trait Meter {
	// Sets whether or not to split channels into individual scales.
	fn split_channels(&mut self, split: bool);

	// Sets the meter's data.
	fn set_data(&mut self, data: &MeterData);

	// Sets the meter's peak.
	fn set_peak(&mut self, peak: Option<u32>);
}

// Stores references to all meter widgets.
pub struct MeterWidgets {
	pub root: gtk::Box,
	
	pub icon: gtk::Image,
	pub label: gtk::Label,
	pub select: gtk::Button,

	pub status: gtk::Button,
	pub status_icon: gtk::Image,

	pub scales_outer: gtk::Box,
	pub scales_inner: gtk::Box,
}

// Builds a single meter scale.
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

pub fn build_scales(pulse: &Shared<PulseController>, data: &MeterData, split: bool) -> gtk::Box {
	let t = data.t;
	let index = data.index;

	let pulse = pulse.clone();
	let scales_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

	if split {
		for _ in 0 .. data.volume.len() {
			let scale = build_scale();
			let pulse = pulse.clone();

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

			scales_box.pack_start(&scale, false, false, 0);
		}
	}
	else {
		let scale = build_scale();
		let channels = data.volume.len();
		let pulse = pulse.clone();
		scale.connect_change_value(move |_, _, value| {
			let mut volumes = ChannelVolumes::default();
			volumes.set_len(channels);
			volumes.set(channels, Volume(value as u32));
			pulse.borrow_mut().set_volume(t, index, volumes);
			gtk::Inhibit(false)
		});
		scales_box.pack_start(&scale, false, false, 0);
	}

	scales_box.show_all();
	scales_box
}

// Builds a meter and returns a struct of all of them.
pub fn build_meter() -> MeterWidgets {
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

	let scales_outer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	let scales_inner = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	scales_outer.pack_start(&scales_inner, true, false, 0);

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
	root.pack_end(&scales_outer, true, true, 2);
	root.pack_start(&top_evt, false, false, 0);

	MeterWidgets {
		root,
		
		icon,
		label,
		select,
		
		status,
		status_icon,

		scales_outer,
		scales_inner
	}
}

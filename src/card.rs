use gtk;
use gtk::prelude::*;
use glib::translate::ToGlib;
use glib::translate::FromGlib;

use crate::shared::Shared;
use crate::pulse_controller::PulseController;


/** Information relating to a Card. */
#[derive(Debug, Clone, Default)]
pub struct CardData {
	pub index: u32,
	
	pub name: String,
	pub icon: String,

	pub profiles: Vec<(String, String)>,
	pub active_profile: String
}

struct CardWidgets {
	root: gtk::Box,
	
	// icon: gtk::Image,
	label: gtk::Label,
	combo: gtk::ComboBoxText,
	// select: gtk::Button,
	// scale_box: gtk::Box,
	// scales: Vec<gtk::Scale>,
	// status: gtk::Button,
	// status_icon: gtk::Image
}


fn build() -> CardWidgets {
	let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	root.set_widget_name("card");

	let inner = gtk::Box::new(gtk::Orientation::Vertical, 0);
	inner.set_border_width(4);
	root.pack_start(&inner, true, true, 4);

	let label_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	label_box.set_border_width(0);
	inner.pack_start(&label_box, false, false, 0);

	let icon = gtk::Image::from_icon_name(Some("audio-card"), gtk::IconSize::LargeToolbar);
	let label = gtk::Label::new(Some("Unknown Card"));

	label_box.pack_start(&icon, false, false, 4);
	label_box.pack_start(&label, false, true, 4);
	
	let combo = gtk::ComboBoxText::new();
	inner.pack_start(&combo, false, false, 8);

	// label.set_widget_name("app_label");

	// label.set_size_request(-1, 42);
	// label.set_justify(gtk::Justification::Center);
	// label.set_ellipsize(pango::EllipsizeMode::End);
	// label.set_line_wrap_mode(pango::WrapMode::WordChar);
	// label.set_max_width_chars(8);
	// label.set_line_wrap(true);
	// label.set_lines(2);


	CardWidgets {
		root,
		// icon,
		label,
		combo
	}
}

pub struct Card {
	pub widget: gtk::Box,
	widgets: CardWidgets,

	data: CardData,
	pulse: Option<Shared<PulseController>>,
	combo_connect_id: Option<glib::signal::SignalHandlerId>,
}

impl Card {
	pub fn new(pulse: Option<Shared<PulseController>>) -> Self {
		let widgets = build();
		Self {
			widget: widgets.root.clone(), widgets,
			data: CardData::default(),
			pulse,
			combo_connect_id: None
		}
	}

	fn disconnect(&mut self) {
		if self.combo_connect_id.is_some() {
			self.widgets.combo.disconnect(glib::signal::SignalHandlerId::from_glib(self.combo_connect_id.as_ref().unwrap().to_glib()));
		}
	}

	fn connect(&mut self) {
		if self.pulse.is_none() { return; }

		let index = self.data.index;
		let pulse = self.pulse.as_ref().unwrap().clone();
		self.combo_connect_id = Some(self.widgets.combo.connect_changed(move |combo| {
			println!("Done~");
			let val = combo.get_active_id().unwrap().as_str().to_owned();
			pulse.borrow_mut().set_card_profile(index, val.as_str());
		}));
	}

	pub fn set_data(&mut self, data: &CardData) {
		if data.index != self.data.index {
			self.data.index = data.index;
			self.disconnect();
			self.connect();
		}

		if data.name != self.data.name {
			self.data.name = data.name.clone();
			self.widgets.label.set_label(self.data.name.as_str());
		}

		if data.profiles.len() != self.data.profiles.len() {
			self.disconnect();
			self.data.profiles = data.profiles.clone();
			self.widgets.combo.remove_all();
			for (i, n) in data.profiles.iter() {
				self.widgets.combo.append(Some(i.as_str()), n.as_str());
			}
			self.connect();
		}

		if data.active_profile != self.data.active_profile {
			self.disconnect();
			self.data.active_profile = data.active_profile.clone();
			self.widgets.combo.set_active_id(Some(self.data.active_profile.as_str()));
			self.connect();
		}
	}
}

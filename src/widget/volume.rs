use once_cell::unsync::OnceCell;

use gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::subclass;
use glib::translate::*;

use std::thread;

glib::glib_wrapper! {
	pub struct Volume(
		Object<subclass::simple::InstanceStruct<VolumePriv>,
		subclass::simple::ClassStruct<VolumePriv>,
		VolumeClass>)
		@extends gtk::Box, gtk::Orientable, gtk::Container, gtk::Widget;

	match fn {
		get_type => || VolumePriv::get_type().to_glib(),
	}
}

impl Volume {
	pub fn new() -> Self {
		glib::Object::new(
			Self::static_type(),
			&[])
		.expect("Failed to create Volume Widget.")
		.downcast().expect("Created Volume Widget could not be downcast.")
	}
}

struct VolumeWidgets {
	app_icon: gtk::Image,
	app_name: gtk::Label,
	// scale: gtk::Scale,
	// volume: gtk::Label
}

pub struct VolumePriv {
	widgets: OnceCell<VolumeWidgets>,
	// muted: bool
}

pub trait VolumeExt {
	fn set_system(&self);
}

impl VolumeExt for Volume {
	fn set_system(&self) {
		let priv_ = VolumePriv::from_instance(self);
		let widgets = priv_.widgets.get().unwrap();

		widgets.app_name.set_label("System");
		widgets.app_icon.set_from_icon_name(Some("preferences-system-sound"), gtk::IconSize::Dnd);

		self.get_style_context().add_class("system");
	}
}

impl VolumePriv {

}


// static PROPERTIES: [subclass::Property; 1] = [
// 	subclass::Property("auto-update", |auto_update| {
// 		glib::ParamSpec::boolean(
// 			"muted", "Muted", "Whether or not the stream is muted.",
// 			false, glib::ParamFlags::READWRITE)
// 	})
// ];

impl ObjectSubclass for VolumePriv {
	const NAME: &'static str = "Volume";
	type ParentType = gtk::Box;
	type Instance = subclass::simple::InstanceStruct<Self>;
	type Class = subclass::simple::ClassStruct<Self>;

	glib_object_subclass!();

	// fn class_init(_: &mut Self::Class) {
	// 	// c.install_properties(&PROPERTIES);
	// 	// c.add_signal(
	// 	// 	"added",
	// 	// 	glib::SignalFlags::RUN_LAST,
	// 	// 	&[Type::U32],
	// 	// 	Type::Unit
	// 	// );
	// }

	fn new() -> Self {
		Self {
			// muted: false,
			widgets: OnceCell::new()
		}
	}
}

impl ObjectImpl for VolumePriv {
	glib::glib_object_impl!();

	fn constructed(&self, obj: &glib::Object) {
		self.parent_constructed(obj);

		let self_ = obj.downcast_ref::<Volume>().unwrap();

		self_.set_orientation(gtk::Orientation::Vertical);
		self_.set_hexpand(false);
		self_.set_size_request(80, -1);

		let app_icon = gtk::Image::from_icon_name(Some("google-chrome"), gtk::IconSize::Dnd);
		self_.pack_start(&app_icon, false, false, 4);

		let app_name = gtk::Label::new(Some("Google Chrome"));

		app_name.set_size_request(-1, 42);
		app_name.set_justify(gtk::Justification::Center);
		app_name.set_ellipsize(pango::EllipsizeMode::End);
		app_name.set_line_wrap_mode(pango::WrapMode::WordChar);
		app_name.set_max_width_chars(8);
		app_name.set_line_wrap(true);
		app_name.set_lines(2);

		self_.pack_start(&app_name, false, true, 0);

		let scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 150.0, 5.0);
		scale.set_increments(5.0, 5.0);
		scale.set_inverted(true);
		scale.set_draw_value(false);
		scale.set_fill_level(2.0);
		scale.set_show_fill_level(true);
		scale.set_restrict_to_fill_level(false);
		scale.add_mark(0.0, gtk::PositionType::Right, Some(""));
		scale.add_mark(100.0, gtk::PositionType::Right, Some(""));
		scale.add_mark(150.0, gtk::PositionType::Right, Some(""));

		self_.set_widget_name("volume");
		self_.pack_start(&scale, true, true, 2);

		// let volume = gtk::Label::new(Some("20%"));
		// // self_.pack_start(&volume, false, false, 2);

		let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

		let button = gtk::Button::with_label("0%");
		let image = gtk::Image::from_icon_name(Some("audio-volume-low-symbolic"), gtk::IconSize::Button);
		button.set_image(Some(&image));
		button.set_always_show_image(true);
		button_box.pack_start(&button, true, false, 0);
		button.set_widget_name("mute_toggle");
		button.get_style_context().add_class("flat");

		let img_ref = image.clone();
		button.connect_clicked(move |button| {
			println!("{:?}", thread::current().name());
			if button.get_style_context().has_class("muted") {
				button.get_style_context().remove_class("muted");
				img_ref.set_from_icon_name(Some("audio-volume-low-symbolic"), gtk::IconSize::Button);
			}
			else {
				button.get_style_context().add_class("muted");
				img_ref.set_from_icon_name(Some("action-unavailable-symbolic"), gtk::IconSize::Button);
			}
		});

		let button_ref = button.clone();
		let image_ref = image.clone();
		scale.connect_value_changed(move |scale| {
			let value = scale.get_value().floor();

			if value >= 100.0 {
				image_ref.set_from_icon_name(Some("audio-volume-high-symbolic"), gtk::IconSize::Button);
			}
			else if value >= 10.0 {
				image_ref.set_from_icon_name(Some("audio-volume-medium-symbolic"), gtk::IconSize::Button);
			}
			else {
				image_ref.set_from_icon_name(Some("audio-volume-low-symbolic"), gtk::IconSize::Button);
			}

			let mut string = value.to_string();
			string.push_str("%");
			button_ref.set_label(string.as_str());
			scale.set_fill_level(scale.get_value() / 2.0);
		});
		
		self_.pack_start(&button_box, false, false, 4);
		
		self_.show_all();

		if self.widgets.set(VolumeWidgets {
			app_name, app_icon
		}).is_err() {
			panic!("Widgets were already set.");
		}
	}

	// fn set_property(&self, _obj: &glib::Object, id: usize, value: &glib::Value) {
	// 	let prop = &PROPERTIES[id];
	// }

	// fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
	// 	let prop = &PROPERTIES[id];
	// }
}

impl BoxImpl for VolumePriv {}
impl ContainerImpl for VolumePriv {}
impl WidgetImpl for VolumePriv {}

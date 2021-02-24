
use once_cell::unsync::OnceCell;

use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

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
	scale: gtk::Scale,
	volume: gtk::Label
}

pub struct VolumePriv {
	widgets: OnceCell<VolumeWidgets>,
	muted: bool
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


static PROPERTIES: [subclass::Property; 1] = [
	subclass::Property("auto-update", |auto_update| {
		glib::ParamSpec::boolean(
			"muted", "Muted", "Whether or not the stream is muted.",
			false, glib::ParamFlags::READWRITE)
	})
];

impl ObjectSubclass for VolumePriv {
	const NAME: &'static str = "Volume";
	type ParentType = gtk::Box;
	type Instance = subclass::simple::InstanceStruct<Self>;
	type Class = subclass::simple::ClassStruct<Self>;

	glib_object_subclass!();

	fn class_init(c: &mut Self::Class) {
		c.install_properties(&PROPERTIES);
		// c.add_signal(
		// 	"added",
		// 	glib::SignalFlags::RUN_LAST,
		// 	&[Type::U32],
		// 	Type::Unit
		// );
	}

	fn new() -> Self {
		Self {
			muted: false,
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
		self_.pack_start(&app_icon, false, false, 8);

		let app_name = gtk::Label::new(Some("Google Chrome"));
		app_name.set_size_request(32, 32);
		app_name.set_line_wrap(true);
		app_name.set_justify(gtk::Justification::Center);
		app_name.set_hexpand(false);
		self_.pack_start(&app_name, false, false, 0);

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

		let volume = gtk::Label::new(Some("20%"));
		// self_.pack_start(&volume, false, false, 2);

		let volume_ref = volume.clone();
		scale.connect_value_changed(move |scale| {
			let mut value = scale.get_value().floor().to_string();
			value.push_str("%");
			volume_ref.set_label(value.as_str());
			scale.set_fill_level(scale.get_value() / 2.0);
		});

		let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
		// button_box.get_style_context().add_class("button_group");

		// let button = gtk::Button::from_icon_name(Some("edit-undo-symbolic"), gtk::IconSize::Button);
		// button_box.pack_start(&button, false, false, 0);
		// let button = gtk::Button::from_icon_name(Some("audio-volume-high-symbolic"), gtk::IconSize::Button);
		let button = gtk::Button::with_label("20%");
		button_box.pack_start(&button, true, false, 0);
		button.get_style_context().add_class("flat");

		button.connect_clicked(|button| {
			if (button.get_style_context().has_class("flat")) {
				button.get_style_context().remove_class("flat");
			}
			else {
				button.get_style_context().add_class("flat");
			}
		});
		// button.set_justify(gtk::Justification::Center);
		// let button = gtk::Button::from_icon_name(Some("applications-system-symbolic"), gtk::IconSize::Button);
		// button_box.pack_start(&button, false, false, 0);
		
		self_.pack_start(&button_box, false, false, 4);
		
		self_.show_all();

		if self.widgets.set(VolumeWidgets {
			app_name, app_icon, scale, volume
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

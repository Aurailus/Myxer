#[macro_use]
extern crate glib;
extern crate gtk;
extern crate gio;

use gtk::prelude::*;
use gio::prelude::*;

use pulse::mainloop::threaded::Mainloop;
use pulse::context::{ Context, FlagSet };
use pulse::proplist::Proplist;
// use pulse::mainloop::api::Mainloop as MainloopTrait;

#[path = "./widget/volume.rs"]
mod volume;
use volume::Volume;
use volume::VolumeExt;
mod shared;
use shared::Shared;

fn build_ui(app: &gtk::Application) {
	let window = gtk::ApplicationWindow::new(app);
	window.set_title("VMix");
	window.set_resizable(false);
	window.set_default_size(500, 300);
	window.set_border_width(4);

	let style = include_str!("./style.css");
	let provider = gtk::CssProvider::new();
	provider.load_from_data(style.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK css provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

	let outer_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	let system_volume = Volume::new();
	system_volume.set_system();
	system_volume.get_style_context().add_class("bordered");
	outer_container.pack_start(&system_volume, false, false, 0);

	let scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
	scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
	scroller.get_style_context().add_class("bordered");
	outer_container.pack_start(&scroller, true, true, 0);

	let inner_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
	scroller.add(&inner_container);

	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);
	let volume_widget = Volume::new();
	inner_container.pack_start(&volume_widget, false, false, 0);

	window.add(&outer_container);
	window.show_all();
}

fn main() {
	let mut proplist = Proplist::new().unwrap();
	proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "VMix")
		.expect("Failed to set Pulse application name.");

	let mainloop = Shared::new(Mainloop::new()
		.expect("Failed to initialize Pulse mainloop."));

	let context = Shared::new(
		Context::new_with_proplist(&*mainloop.borrow(), "VMix Context", &proplist)
		.expect("Failed to create Pulse context."));

	/* Pass forward relevant states to the loop below. */ {
		let mainloop_ref = mainloop.clone();
		let context_ref = context.clone();
		context.borrow_mut().set_state_callback(Some(Box::new(move || {
			match unsafe { (*context_ref.as_ptr()).get_state() } {
				pulse::context::State::Ready |
				pulse::context::State::Failed |
				pulse::context::State::Terminated => {
					unsafe { (*mainloop_ref.as_ptr()).signal(false); }
				},
				_ => {},
			}
		})));
	}

	context.borrow_mut().connect(None, FlagSet::NOFLAGS, None)
		.expect("Failed to connect context");

	mainloop.borrow_mut().lock();
	mainloop.borrow_mut().start().expect("Failed to start mainloop");

	/* Wait for a valid state to be passed forward */
	loop {
		let mut ctx = context.borrow_mut();
		match ctx.get_state() {
			pulse::context::State::Ready => {
				ctx.set_state_callback(None);
				break;
			},
			pulse::context::State::Failed |
			pulse::context::State::Terminated => {
				eprintln!("Context state failed/terminated, quitting...");
				mainloop.borrow_mut().unlock();
				mainloop.borrow_mut().stop();
				return;
			},
			_ => { mainloop.borrow_mut().wait(); },
		}
	}

	context.borrow_mut().set_event_callback(Some(Box::new(|event, _o| {
		println!("Other!");
		println!("{}", event);
	})));

	context.borrow_mut().introspect().get_client_info_list(|info| {
		println!("Callback!");
		println!("{:?}", info);
	});

	let app = gtk::Application::new(Some("com.aurailus.vmix"), Default::default())
		.expect("Failed to initialize GTK application.");

	app.connect_activate(|app| build_ui(app));

	app.run(&[]);

	mainloop.borrow_mut().unlock();
	mainloop.borrow_mut().stop();
}

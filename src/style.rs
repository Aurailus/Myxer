#![allow(deprecated)]

use gtk::prelude::*;

pub fn style(window: &gtk::ApplicationWindow) {
	let provider = gtk::CssProvider::new();

	let mut s = String::new();

	let mut add_color = |identifier: &str, color: &gdk::RGBA| {
		s.push_str("@define-color ");
		s.push_str(identifier);
		s.push_str(" ");
		s.push_str(colorsys::Rgb::new(color.red * 255.0, color.green * 255.0, color.blue * 255.0, None).to_css_string().as_str());
		s.push_str(";\n");
	};

	let row = gtk::ListBoxRow::new();
	add_color("scale_color", &row.get_style_context().get_background_color(gtk::StateFlags::SELECTED));
	add_color("background_color", &window.get_style_context().get_background_color(gtk::StateFlags::NORMAL));
	add_color("foreground_color", &row.get_style_context().get_color(gtk::StateFlags::NORMAL));

	let style = include_str!("./style.css");
	s.push_str(style);

	provider.load_from_data(s.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK CSS provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

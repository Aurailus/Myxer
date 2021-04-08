/*!
 * Handles loading and generating custom styles for the Myxer application.
 */

#![allow(deprecated)]

use gtk::prelude::*;


/**
 * Applies application-specific styles to the specified window.
 * Generates a few GTK widgets to steal their theme colors using deprecated methods.
 * Unfortunately, it seems that there is no other way to pull theme colors dynamically,
 * which seems odd, perhaps I've overlooked something.
 *
 * * `window` - The main Myxer application window.
 */

pub fn style(window: &gtk::ApplicationWindow) {
	let provider = gtk::CssProvider::new();

	let mut s = String::new();

	let mut add_color = |identifier: &str, color: &gdk::RGBA| {
		s.push_str("@define-color ");
		s.push_str(identifier);
		s.push_str(" ");
		s.push_str(&colorsys::Rgb::new(color.red * 255.0, color.green * 255.0, color.blue * 255.0, None).to_css_string());
		s.push_str(";\n");
	};

	let row = gtk::ListBoxRow::new();
	let button = gtk::Button::new();
	add_color("scale_color", &row.get_style_context().get_background_color(gtk::StateFlags::SELECTED));
	add_color("background_color", &window.get_style_context().get_background_color(gtk::StateFlags::NORMAL));
	add_color("foreground_color", &button.get_style_context().get_color(gtk::StateFlags::NORMAL));

	s.push_str(STYLE);

	provider.load_from_data(s.as_bytes()).expect("Failed to load CSS.");
	gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().expect("Error initializing GTK CSS provider."),
		&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

/**
 * The custom stylesheet used by Myxer.
 */
 
const STYLE: &'static str = r#"
.title {
	padding-left: 3px;
}

.pad_side {
	padding-left: 3px;
	padding-right: 3px;
}

headerbar box:last-child {
	padding-left: 0;
}

headerbar button#preferences.image-button {
	border-radius: 16px;
	margin-right: -6px;
	padding: 0px;
	margin-top: 8px;
	margin-bottom: 8px;
}

headerbar #preferences image {
	opacity: .7;
	-gtk-icon-transform: scale(0.75);
}

headerbar box:last-child separator {
	border-width: 0;
	border-image-source: none;
}

#meter scale.visualizer trough highlight {
	background: none;
	background-color: alpha(@scale_color, 0.4);
}

#meter scale.visualizer trough fill {
	background: none;
	background-color: alpha(@scale_color, 1);
}

#meter {
	padding-top: 3px;
	padding-bottom: 3px;
}

#meter #top {
	padding: 0;
	margin: 2px;
}

#meter #app_label {
	padding: 0 3px;
}

#meter #app_select {
	padding: 0;
	margin: -1px 2px;
}

#meter #app_select #app_label {
	padding: 0;
}

#meter #mute_toggle image {
	margin-top: 1px;
}

#meter #mute_toggle label {
	margin-left: 3px;
}

#meter #mute_toggle.muted {
	border-color: darker(@background_color);
}

#meter #mute_toggle.muted {
	color: alpha(@foreground_color, 0.4);
}

undershoot {
	background: none;
}

#card {
	margin-top: -3px;
	margin-bottom: -6px;
}

#card:first-child {
	margin-top: 6px;
}
"#;

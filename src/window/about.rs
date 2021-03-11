use gtk::prelude::*;

pub fn about() {
	let about = gtk::AboutDialog::new();
	about.set_logo_icon_name(Some("multimedia-volume-control"));
	about.set_program_name("Myxer");
	about.set_version(Some("1.1.1"));
	about.set_comments(Some("A modern Volume Mixer for PulseAudio."));
	about.set_website(Some("https://myxer.aurailus.com"));
	about.set_copyright(Some("© 2021 Auri Collings"));
	about.set_license_type(gtk::License::Gpl30);
	about.add_credit_section("Created by", &[ "Auri Collings" ]);
	about.add_credit_section("libpulse-binding by", &[ "Lyndon Brown" ]);

	about.connect_response(|about, _| about.close());
	about.run();
}

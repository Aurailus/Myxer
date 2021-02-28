use gtk::prelude::*;

pub struct Notebook {
	pub widget: gtk::Notebook,
	tabs: Vec<gtk::Box>
}

impl Notebook {
	pub fn new() -> Self {
		Notebook {
			widget: gtk::Notebook::new(),
			tabs: Vec::new()
		}
	}

	pub fn add_tab(&mut self, label: &str, widget: gtk::Widget) -> u32 {
		let label = gtk::Label::new(Some(label));
		let tab = gtk::Box::new(gtk::Orientation::Horizontal, 0);

		tab.pack_start(&label, false, false, 0);
		tab.show_all();

		self.widget.set_show_tabs(false);
		let index = self.widget.append_page(&widget, Some(&tab));
		self.tabs.push(tab);
		index
	}
}

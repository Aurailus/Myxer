use std::collections::HashMap;

use gtk::prelude::*;

use crate::card::Card;
use crate::shared::Shared;
use crate::pulse_controller::PulseController;

struct Container {
	live: bool
}

struct Cards {
	cards: HashMap<u32, Card>,
	cards_box: gtk::Box,
}

impl Cards {
	pub fn new() -> Self {
		Cards { cards: HashMap::new(), cards_box: gtk::Box::new(gtk::Orientation::Vertical, 8) }
	}
}

pub struct Profiles {
	cards: Shared<Cards>,
	pulse: Shared<PulseController>,

	live: Shared<Container>
}

impl Profiles {
	pub fn new(parent: &gtk::ApplicationWindow, pulse: &Shared<PulseController>) -> Self {
		let dialog = gtk::Dialog::with_buttons(Some("Card Profiles"), Some(parent), gtk::DialogFlags::all(), &[]);
		dialog.set_border_width(0);

		let live = Shared::new(Container { live: true });
		dialog.connect_response(|s, _| s.emit_close());
		let live_clone = live.clone();
		dialog.connect_close(move |_| live_clone.borrow_mut().live = false);

		let geom = gdk::Geometry {
			min_width: 450, min_height: 550,
			max_width: 450, max_height: 10000,
			base_width: -1, base_height: -1,
			width_inc: -1, height_inc: -1,
			min_aspect: 0.0, max_aspect: 0.0,
			win_gravity: gdk::Gravity::Center
		};

		dialog.set_geometry_hints::<gtk::Dialog>(None, Some(&geom), gdk::WindowHints::MIN_SIZE | gdk::WindowHints::MAX_SIZE);
		let cards = Shared::new(Cards::new());
		
		let scroller = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
		scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
		dialog.get_content_area().pack_start(&scroller, true, true, 0);
		dialog.get_content_area().set_border_width(0);
		scroller.add(&cards.borrow().cards_box);

		dialog.show_all();

		Self {
			live,
			cards,
			pulse: pulse.clone()
		}
	}

	pub fn update(&mut self) -> bool {
		// println!("Update");

		let pulse = self.pulse.borrow_mut();
		let mut cards = self.cards.borrow_mut();
		for (index, data) in pulse.cards.iter() {
			let cards_box = cards.cards_box.clone();
			
			let card = cards.cards.entry(*index).or_insert_with(|| Card::new(Some(self.pulse.clone())));
			if card.widget.get_parent().is_none() { cards_box.pack_start(&card.widget, false, false, 0); }
			card.set_data(&data);
		}

		cards.cards_box.show_all();

		// let source_outputs_box = meters.source_outputs_box.clone();
		// meters.source_outputs.retain(|index, meter| {
		// 	let keep = pulse.source_outputs.contains_key(index);
		// 	if !keep { source_outputs_box.remove(&meter.widget); }
		// 	keep
		// });

		self.live.borrow().live
	}
}

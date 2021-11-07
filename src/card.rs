/*!
 * A widget representing a pulseaudio sound card.
 * Has a dropdown that allows the current card profile to be changed.
 */

use std::cell::RefCell;

use gtk::prelude::*;

use crate::pulse::Pulse;
use crate::shared::Shared;

/**
 * Holds a Card's data.
 */

#[derive(Debug, Clone, Default)]
pub struct CardData {
    pub index: u32,

    pub name: String,
    pub icon: String,

    pub profiles: Vec<(String, String)>,
    pub active_profile: String,
}

/**
 * Holds a Card's widgets.
 */

struct CardWidgets {
    root: gtk::Box,

    label: gtk::Label,
    combo: gtk::ComboBoxText,
}

/**
 * A widget representing a pulseaudio sound card.
 * Has a dropdown that allows the current card profile to be changed.
 */

pub struct Card {
    pub widget: gtk::Box,
    widgets: CardWidgets,

    data: CardData,
    pulse: Option<Shared<Pulse>>,
    combo_connect_id: RefCell<Option<glib::signal::SignalHandlerId>>,
}

impl Card {
    /**
     * Constructs the GTK widgets required for the card widget.
     */

    fn build() -> CardWidgets {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.set_widget_name("card");

        let inner = gtk::Box::new(gtk::Orientation::Vertical, 0);
        inner.set_border_width(3);
        root.pack_start(&inner, true, true, 3);

        let label_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        label_box.set_border_width(0);
        inner.pack_start(&label_box, false, false, 3);

        let icon = gtk::Image::from_icon_name(Some("audio-card"), gtk::IconSize::LargeToolbar);
        let label = gtk::Label::new(Some("Unknown Card"));

        label_box.pack_start(&icon, false, false, 3);
        label_box.pack_start(&label, false, true, 3);

        let combo = gtk::ComboBoxText::new();
        inner.pack_start(&combo, false, false, 6);

        CardWidgets { root, label, combo }
    }

    /**
     * Creates a new card widget.
     */

    pub fn new(pulse: Option<Shared<Pulse>>) -> Self {
        let widgets = Card::build();
        Self {
            widget: widgets.root.clone(),
            widgets,
            data: CardData::default(),
            pulse,
            combo_connect_id: RefCell::new(None),
        }
    }

    /**
     * Disconnect's a card widget from the Pulse instance,
     * in the event that significant information has changed for the callbacks to be invalid.
     */

    fn disconnect(&self) {
        if let Some(id) = self.combo_connect_id.borrow_mut().take() {
            self.widgets.combo.disconnect(id);
        }
    }

    /**
     * Connects a callback to the widget's dropdown,
     * so that changing the selected option changes the card's profile.
     * If the Pulse instance is None, the callback is not bound.
     */

    fn connect(&mut self) {
        if self.pulse.is_none() {
            return;
        }

        let index = self.data.index;
        let pulse = self.pulse.as_ref().unwrap().clone();
        self.combo_connect_id
            .replace(Some(self.widgets.combo.connect_changed(move |combo| {
                pulse
                    .borrow_mut()
                    .set_card_profile(index, &String::from(combo.active_id().unwrap()));
            })));
    }

    /**
     * Updates the Card's data, and visually refreshes the required components.
     */

    pub fn set_data(&mut self, data: &CardData) {
        if data.index != self.data.index {
            self.data.index = data.index;
            self.disconnect();
            self.connect();
        }

        if data.name != self.data.name {
            self.data.name = data.name.clone();
            self.widgets.label.set_label(&self.data.name);
        }

        if data.profiles.len() != self.data.profiles.len() {
            self.disconnect();
            self.data.profiles = data.profiles.clone();
            self.widgets.combo.remove_all();
            for (i, n) in &data.profiles {
                self.widgets.combo.append(Some(&i), &n);
            }
            self.connect();
        }

        if data.active_profile != self.data.active_profile {
            self.disconnect();
            self.data.active_profile = data.active_profile.clone();
            self.widgets
                .combo
                .set_active_id(Some(&self.data.active_profile));
            self.connect();
        }
    }
}

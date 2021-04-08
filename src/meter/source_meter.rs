/*!
 * A specialized meter for representing sources.
 */

use gtk;
use gtk::prelude::*;
use glib::translate::{ ToGlib, FromGlib };

use crate::pulse::Pulse;
use crate::shared::Shared;
use super::meter::{ Meter, MeterWidgets, MeterData };
use super::meter::{ MAX_NATURAL_VOL, MAX_SCALE_VOL, INPUT_ICONS };


/**
 * A meter widget representing a source.
 * Has interactions to allow changing the active source.
 */

pub struct SourceMeter {
	pub widget: gtk::Box,

	data: MeterData,
	widgets: MeterWidgets,
	pulse: Shared<Pulse>,

	split: bool,
	peak: Option<u32>,

	l_id: Option<glib::signal::SignalHandlerId>,
	s_id: Option<glib::signal::SignalHandlerId>,
}

impl SourceMeter {

	/**
	 * Creates a new SourceMeter.
	 */

	pub fn new(pulse: Shared<Pulse>) -> Self {
		let widgets = Meter::build_meter();

		Self {
			widget: widgets.root.clone(),
			
			pulse,
			widgets,
			data: MeterData::default(),

			split: false, peak: None, l_id: None, s_id: None
		}
	}


	/**
	 * Rebuilds widgets that are dependent on the Pulse instance or the source index.
	 * Reconnects the widgets to the Pulse instance, if one is provided.
	 */

	fn rebuild_widgets(&mut self) {
		let scales = Meter::build_scales(&self.pulse, &self.data, self.split);
		self.widgets.scales_outer.remove(&self.widgets.scales_inner);
		self.widgets.scales_outer.pack_start(&scales, true, false, 0);
		self.widgets.scales_inner = scales;
		self.update_widgets();

		let t 		= self.data.t;
		let index = self.data.index;
		let pulse = self.pulse.clone();

		if self.s_id.is_some() { self.widgets.status.disconnect(
			glib::signal::SignalHandlerId::from_glib(self.s_id.as_ref().unwrap().to_glib())) }
		self.s_id = Some(self.widgets.status.connect_clicked(move |status| {
			pulse.borrow_mut().set_muted(t, index,
				!status.get_style_context().has_class("muted"));
		}));

		let pulse = self.pulse.clone();
		let index = self.data.index;

		if self.l_id.is_some() { self.widgets.app_button.disconnect(
			glib::signal::SignalHandlerId::from_glib(self.l_id.as_ref().unwrap().to_glib())) }
		self.l_id = Some(self.widgets.app_button.connect_clicked(move |trigger| {
			SourceMeter::show_popup(&trigger, &pulse, index);
		}));
	}


	/**
	 * Updates each scale widget to reflect the current volume level.
	 */

	fn update_widgets(&mut self) {
		for (i, v) in self.data.volume.get().iter().enumerate() {
			if let Some(scale) = self.widgets.scales_inner.get_children().get(i) {
				let scale = scale.clone().downcast::<gtk::Scale>().expect("Scales box has non-scale children.");
				scale.set_sensitive(!self.data.muted);
				scale.set_value(v.0 as f64);
			}
		}
	}


	/**
	 * Shows a popup menu on the top button, with items to set
	 * the Sink as default, and change the visible source.
	 */

	fn show_popup(trigger: &gtk::Button, pulse_shr: &Shared<Pulse>, index: u32) {
		let pulse = pulse_shr.borrow_mut();
		let root = gtk::PopoverMenu::new();
		root.set_border_width(6);

		let menu = gtk::Box::new(gtk::Orientation::Vertical, 0);
		menu.set_size_request(132, -1);
		root.add(&menu);
		
		// let split_channels = gtk::ModelButton::new();
		// split_channels.set_property_text(Some("Split Channels"));
		// split_channels.set_action_name(Some("app.split_channels"));
		// menu.add(&split_channels);

		let set_default = gtk::ModelButton::new();
		set_default.set_property_role(gtk::ButtonRole::Check);
		set_default.set_property_text(Some("Set as Default"));
		set_default.set_property_active(pulse.default_source == index);
		set_default.set_sensitive(pulse.default_source != index);
			
		let pulse_clone = pulse_shr.clone();
		set_default.connect_clicked(move |set_default| {
			pulse_clone.borrow_mut().set_default_source(index);
			set_default.set_property_active(true);
			set_default.set_sensitive(false);
		});
		menu.add(&set_default);

		if pulse.sources.len() >= 2 {
			menu.pack_start(&gtk::SeparatorMenuItem::new(), false, false, 3);

			let label = gtk::Label::new(Some("Visible Input"));
			label.set_sensitive(false);
			menu.pack_start(&label, true, true, 3);
			
			for (i, v) in &pulse.sources {
				let button = gtk::ModelButton::new();
				button.set_property_role(gtk::ButtonRole::Radio);
				button.set_property_active(v.data.index == index);
				let button_label = gtk::Label::new(Some(&v.data.description));
				button_label.set_ellipsize(pango::EllipsizeMode::End);
				button_label.set_max_width_chars(18);
				button.get_child().unwrap().downcast::<gtk::Box>().unwrap().add(&button_label);

				let i = *i;
				let root = root.clone();
				let pulse_clone = pulse_shr.clone();
				button.connect_clicked(move |_| {
					pulse_clone.borrow_mut().set_active_source(i);
					root.popdown();
				});
				
				menu.add(&button);
			}
		}

		for child in &root.get_children() { child.show_all(); }
		root.set_relative_to(Some(trigger));
		root.popup();
	}
}

impl Meter for SourceMeter {
	fn get_index(&self) -> u32 {
		self.data.index
	}
	
	fn split_channels(&mut self, split: bool) {
		if self.split == split { return }
		self.split = split;
		self.rebuild_widgets();
	}

	fn set_data(&mut self, data: &MeterData) {
		let volume_old = self.data.volume;
		let volume_changed = data.volume != volume_old;

		if data.t != self.data.t || data.index != self.data.index || data.volume.len() != self.data.volume.len() {
			self.data.t = data.t;
			self.data.volume = data.volume;
			self.data.index = data.index;
			self.rebuild_widgets();
		}

		if data.icon != self.data.icon {
			self.data.icon = data.icon.clone();
			self.widgets.icon.set_from_icon_name(Some(&self.data.icon), gtk::IconSize::Dnd);
		}

		if data.name != self.data.name {
			self.data.name = data.name.clone();
			self.rebuild_widgets();
		}

		if data.description != self.data.description {
			self.data.description = data.description.clone();
			self.widgets.label.set_label(&self.data.description);
			self.widgets.app_button.set_tooltip_text(Some(&self.data.description));
		}

		if volume_changed || data.muted != self.data.muted {
			self.data.volume = data.volume;
			self.data.muted = data.muted;
			self.update_widgets();

			let status_vol = self.data.volume.max().0;

			self.widgets.status_icon.set_from_icon_name(Some(INPUT_ICONS[
				if self.data.muted { 0 } else if status_vol >= MAX_NATURAL_VOL { 3 }
				else if status_vol >= MAX_NATURAL_VOL / 2 { 2 } else { 1 }]), gtk::IconSize::Button);

			let mut vol_scaled = ((status_vol as f64) / MAX_NATURAL_VOL as f64 * 100.0).round() as u8;
			if vol_scaled > 150 { vol_scaled = 150 }

			let mut string = vol_scaled.to_string();
			string.push_str("%");
			self.widgets.status.set_label(&string);

			let status_ctx = self.widgets.status.get_style_context();
			if self.data.muted { status_ctx.add_class("muted") }
			else { status_ctx.remove_class("muted") }
		}
	}

	fn set_peak(&mut self, peak: Option<u32>) {
		if self.peak != peak {
			self.peak = peak;

			if self.peak.is_some() {
				for (i, s) in self.widgets.scales_inner.get_children().iter().enumerate() {
					let s = s.clone().downcast::<gtk::Scale>().expect("Scales box has non-scale children.");
					let peak_scaled = self.peak.unwrap() as f64 * (self.data.volume.get()[i].0 as f64 / MAX_SCALE_VOL as f64);
					s.set_fill_level(peak_scaled as f64);
					s.set_show_fill_level(!self.data.muted && peak_scaled > 0.5);
					s.get_style_context().add_class("visualizer");
				}
			}
			else {
				for s in &self.widgets.scales_inner.get_children() {
					let s = s.clone().downcast::<gtk::Scale>().expect("Scales box has non-scale children.");
					s.set_show_fill_level(false);
					s.get_style_context().remove_class("visualizer");
				}
			}
		}
	}
}

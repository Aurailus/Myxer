/*!
 * A specialized meter for representing application streams.
 */

use std::cell::RefCell;

use gtk::prelude::*;

use super::base_meter::{Meter, MeterData, MeterWidgets};
use super::base_meter::{INPUT_ICONS, MAX_NATURAL_VOL, MAX_SCALE_VOL, OUTPUT_ICONS};
use crate::pulse::{Pulse, StreamType};
use crate::shared::Shared;

/**
 * A meter widget representing an application stream,
 * be it a sink input or a source output.
 */

pub struct StreamMeter {
    pub widget: gtk::Box,

    data: MeterData,
    widgets: MeterWidgets,
    pulse: Shared<Pulse>,

    split: bool,
    peak: Option<u32>,

    b_id: RefCell<Option<glib::signal::SignalHandlerId>>,
}

impl StreamMeter {
    /**
     * Creates a new StreamMeter.
     */

    pub fn new(pulse: Shared<Pulse>) -> Self {
        let widgets = <dyn Meter>::build_meter();
        Self {
            widget: widgets.root.clone(),

            pulse,
            widgets,
            data: MeterData::default(),

            split: false,
            peak: None,
            b_id: RefCell::new(None),
        }
    }

    /**
     * Rebuilds widgets that are dependent on the Pulse instance or the stream index.
     * Reconnects the widgets to the Pulse instance, if one is provided.
     */

    fn rebuild_widgets(&mut self) {
        let scales = <dyn Meter>::build_scales(&self.pulse, &self.data, self.split);
        self.widgets.scales_outer.remove(&self.widgets.scales_inner);
        self.widgets
            .scales_outer
            .pack_start(&scales, true, false, 0);
        self.widgets.scales_inner = scales;
        self.update_widgets();

        let t = self.data.t;
        let index = self.data.index;
        let pulse = self.pulse.clone();

        if let Some(id) = self.b_id.borrow_mut().take() {
            self.widgets.status.disconnect(id)
        }
        self.b_id
            .replace(Some(self.widgets.status.connect_clicked(move |status| {
                pulse
                    .borrow_mut()
                    .set_muted(t, index, !status.style_context().has_class("muted"));
            })));
    }

    /**
     * Updates each scale widget to reflect the current volume level.
     */

    fn update_widgets(&mut self) {
        for (i, v) in self.data.volume.get().iter().enumerate() {
            if let Some(scale) = self.widgets.scales_inner.children().get(i) {
                let scale = scale
                    .clone()
                    .downcast::<gtk::Scale>()
                    .expect("Scales box has non-scale children.");
                scale.set_value(if self.data.muted { 0.0 } else { v.0 as f64 });
            }
        }
    }
}

impl Meter for StreamMeter {
    fn get_index(&self) -> u32 {
        self.data.index
    }

    fn split_channels(&mut self, split: bool) {
        if self.split == split {
            return;
        }
        self.split = split;
        self.rebuild_widgets();
    }

    fn set_data(&mut self, data: &MeterData) {
        let volume_old = self.data.volume;
        let volume_changed = data.volume != volume_old;

        if data.t != self.data.t
            || data.index != self.data.index
            || data.volume.len() != self.data.volume.len()
        {
            self.data.t = data.t;
            self.data.volume = data.volume;
            self.data.index = data.index;
            self.rebuild_widgets();
        }

        if data.icon != self.data.icon {
            self.data.icon = data.icon.clone();
            self.widgets
                .icon
                .set_from_icon_name(Some(&self.data.icon), gtk::IconSize::Dnd);
        }

        if data.name != self.data.name {
            self.data.name = data.name.clone();
        }

        if data.description != self.data.description {
            self.data.description = data.description.clone();
            self.widgets.label.set_label(&self.data.description);
            self.widgets
                .app_button
                .set_tooltip_text(Some(&self.data.description));
        }

        if volume_changed || data.muted != self.data.muted {
            self.data.volume = data.volume;
            self.data.muted = data.muted;
            self.update_widgets();

            let status_vol = if self.data.muted {
                0
            } else {
                self.data.volume.max().0
            };

            let &icons = if self.data.t == StreamType::Sink || self.data.t == StreamType::SinkInput
            {
                &OUTPUT_ICONS
            } else {
                &INPUT_ICONS
            };

            self.widgets.status_icon.set_from_icon_name(
                Some(
                    icons[if status_vol == 0 {
                        0
                    } else if status_vol >= MAX_NATURAL_VOL {
                        3
                    } else if status_vol >= MAX_NATURAL_VOL / 2 {
                        2
                    } else {
                        1
                    }],
                ),
                gtk::IconSize::Button,
            );

            let mut vol_scaled =
                ((status_vol as f64) / MAX_NATURAL_VOL as f64 * 100.0).round() as u8;
            if vol_scaled > 150 {
                vol_scaled = 150
            }

            let mut string = vol_scaled.to_string();
            string.push('%');

            self.widgets.status.set_label(&string);

            if status_vol == 0 {
                self.widgets.status.style_context().add_class("muted")
            } else {
                self.widgets.status.style_context().remove_class("muted")
            }
        }
    }

    fn set_peak(&mut self, peak: Option<u32>) {
        if self.peak != peak {
            self.peak = peak;

            if self.peak.is_some() {
                for (i, s) in self.widgets.scales_inner.children().iter().enumerate() {
                    let s = s
                        .clone()
                        .downcast::<gtk::Scale>()
                        .expect("Scales box has non-scale children.");
                    let peak_scaled = self.peak.unwrap() as f64
                        * (self.data.volume.get()[i].0 as f64 / MAX_SCALE_VOL as f64);
                    s.set_fill_level(peak_scaled as f64);
                    s.set_show_fill_level(!self.data.muted && peak_scaled > 0.5);
                    s.style_context().add_class("visualizer");
                }
            } else {
                for s in &self.widgets.scales_inner.children() {
                    let s = s
                        .clone()
                        .downcast::<gtk::Scale>()
                        .expect("Scales box has non-scale children.");
                    s.set_show_fill_level(false);
                    s.style_context().remove_class("visualizer");
                }
            }
        }
    }
}

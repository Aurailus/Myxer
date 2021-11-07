/*!
 * Contains constants and base traits for specialized meter widgets.
 */

use gtk::prelude::*;
use libpulse::volume::{ChannelVolumes, Volume};

use crate::pulse::{Pulse, StreamType};
use crate::shared::Shared;

/** The maximum natural volume, i.e. 100% */
pub const MAX_NATURAL_VOL: u32 = 65536;

/** The maximum scale volume, i.e. 150% */
pub const MAX_SCALE_VOL: u32 = (MAX_NATURAL_VOL as f64 * 1.5) as u32;

/** The increment step of the scale, e.g. how far it moves when you press up & down. */
pub const SCALE_STEP: f64 = MAX_NATURAL_VOL as f64 / 20.0;

/** The icon names for the input meter statuses. */
pub const INPUT_ICONS: [&str; 4] = [
    "microphone-sensitivity-muted-symbolic",
    "microphone-sensitivity-low-symbolic",
    "microphone-sensitivity-medium-symbolic",
    "microphone-sensitivity-high-symbolic",
];

/** The icon names for the output meter statuses. */
pub const OUTPUT_ICONS: [&str; 4] = [
    "audio-volume-muted-symbolic",
    "audio-volume-low-symbolic",
    "audio-volume-medium-symbolic",
    "audio-volume-high-symbolic",
];

/**
 * Holds a Meter widget's display data.
 */

#[derive(Debug, Clone, Default)]
pub struct MeterData {
    pub t: StreamType,
    pub index: u32,

    pub name: String,
    pub icon: String,
    pub description: String,

    pub volume: ChannelVolumes,
    pub muted: bool,
}

/**
 * Holds references to a Meter's widgets.
 */

pub struct MeterWidgets {
    pub root: gtk::Box,

    pub icon: gtk::Image,
    pub label: gtk::Label,
    pub select: gtk::Button,
    pub app_button: gtk::Button,

    pub status: gtk::Button,
    pub status_icon: gtk::Image,

    pub scales_outer: gtk::Box,
    pub scales_inner: gtk::Box,
}

/**
 * The base trait for all Meter widgets.
 * A Meter widget is a visual control consisting of a name,
 * app icon, volume slider, and mute button.
 */

pub trait Meter {
    /**
     * Gets the meter's underlying stream index.
     */

    fn get_index(&self) -> u32;

    /**
     * Sets whether or not to split channels into individual meters.
     *
     * * `split` - Whether or not channels should be separated.
     */

    fn split_channels(&mut self, split: bool);

    /**
     * Updates the meter's data, and visually refreshes the required widgets.
     */

    fn set_data(&mut self, data: &MeterData);

    /**
     * Sets the meter's current peak.
     *
     * * `peak` - The meter's peak, or None if no peak indicator should be shown.
     */

    fn set_peak(&mut self, peak: Option<u32>);
}

impl dyn Meter {
    /**
     * Builds a scale. This may be for a single channel, or all channels.
     */

    fn build_scale() -> gtk::Scale {
        let scale = gtk::Scale::with_range(
            gtk::Orientation::Vertical,
            0.0,
            MAX_SCALE_VOL as f64,
            SCALE_STEP,
        );

        scale.set_inverted(true);
        scale.set_draw_value(false);
        scale.set_increments(SCALE_STEP, SCALE_STEP);
        scale.set_restrict_to_fill_level(false);

        scale.add_mark(0.0, gtk::PositionType::Right, Some(""));
        scale.add_mark(MAX_SCALE_VOL as f64, gtk::PositionType::Right, Some(""));
        scale.add_mark(MAX_NATURAL_VOL as f64, gtk::PositionType::Right, Some(""));

        scale
    }

    /**
     * Builds the required scales for a Meter.
     * This may be one or more, depending on the state of the `split` variable.
     *
     * * `pulse` - The pulse store to bind events to.
     * * `data`  - The meter data to base the scales off of.
     * * `split` - Whether or not one merged bar should be created, or individual bars for each channel.
     */

    pub fn build_scales(pulse: &Shared<Pulse>, data: &MeterData, split: bool) -> gtk::Box {
        let t = data.t;
        let index = data.index;

        let pulse = pulse.clone();
        let scales_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        if split {
            for _ in 0..data.volume.len() {
                let scale = <dyn Meter>::build_scale();
                let pulse = pulse.clone();

                scale.connect_change_value(move |scale, _, val| {
                    let parent = scale.parent().unwrap().downcast::<gtk::Box>().unwrap();
                    let children = parent.children();

                    let mut volumes = ChannelVolumes::default();

                    // So, if you're wondering why rev() is necessary or why I set the len after or why this is horrible in general,
                    // Check out libpulse_binding::volumes::ChannelVolumes::set, and you'll see ._.
                    for (i, w) in children.iter().enumerate().rev() {
                        let s = w.clone().downcast::<gtk::Scale>().unwrap();
                        let value = if *scale == s { val } else { s.value() };
                        let volume = Volume(value as u32);
                        volumes.set(i as u8 + 1, volume);
                    }

                    volumes.set_len(children.len() as u8);

                    let pulse = pulse.borrow_mut();
                    pulse.set_volume(t, index, volumes);
                    if volumes.max().0 > 0 {
                        pulse.set_muted(t, index, false);
                    }
                    gtk::Inhibit(false)
                });

                scales_box.pack_start(&scale, false, false, 0);
            }
        } else {
            let scale = <dyn Meter>::build_scale();
            let channels = data.volume.len();
            let pulse = pulse.clone();
            scale.connect_change_value(move |_, _, value| {
                let mut volumes = ChannelVolumes::default();
                volumes.set_len(channels);
                volumes.set(channels, Volume(value as u32));
                let pulse = pulse.borrow_mut();
                pulse.set_volume(t, index, volumes);
                if volumes.max().0 > 0 {
                    pulse.set_muted(t, index, false);
                }
                gtk::Inhibit(false)
            });
            scales_box.pack_start(&scale, false, false, 0);
        }

        scales_box.show_all();
        scales_box
    }

    /**
     * Initializes all of the Widgets to make a meter, and returns them.
     */

    pub fn build_meter() -> MeterWidgets {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.set_widget_name("meter");

        root.set_orientation(gtk::Orientation::Vertical);
        root.set_hexpand(false);
        root.set_size_request(86, -1);

        let app_button = gtk::Button::new();
        app_button.set_widget_name("top");
        app_button.style_context().add_class("flat");
        let label_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        app_button.add(&label_container);

        let icon =
            gtk::Image::from_icon_name(Some("audio-volume-muted-symbolic"), gtk::IconSize::Dnd);

        let label = gtk::Label::new(Some("Unknown"));
        label.set_widget_name("app_label");

        label.set_size_request(-1, 42);
        label.set_justify(gtk::Justification::Center);
        label.set_ellipsize(pango::EllipsizeMode::End);
        label.set_line_wrap_mode(pango::WrapMode::WordChar);
        label.set_max_width_chars(8);
        label.set_line_wrap(true);
        label.set_lines(2);

        label_container.pack_end(&label, false, true, 0);
        label_container.pack_end(&icon, false, false, 3);

        let select = gtk::Button::new();
        select.set_widget_name("app_select");
        select.style_context().add_class("flat");

        let scales_outer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let scales_inner = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        scales_outer.pack_start(&scales_inner, true, false, 0);

        let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let status_icon =
            gtk::Image::from_icon_name(Some("audio-volume-muted-symbolic"), gtk::IconSize::Button);

        let status = gtk::Button::new();
        status.set_widget_name("mute_toggle");
        status_box.pack_start(&status, true, false, 0);

        status.set_image(Some(&status_icon));
        status.set_always_show_image(true);
        status.style_context().add_class("flat");
        status.style_context().add_class("muted");

        root.pack_end(&status_box, false, false, 3);
        root.pack_end(&scales_outer, true, true, 2);
        root.pack_start(&app_button, false, false, 0);

        MeterWidgets {
            root,

            icon,
            label,
            select,
            app_button,

            status,
            status_icon,

            scales_outer,
            scales_inner,
        }
    }
}

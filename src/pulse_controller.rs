use slice_as_array::{ slice_as_array, slice_as_array_transmute };

use pulse::def::BufferAttr;
use pulse::proplist::Proplist;
use pulse::callbacks::ListResult;
use pulse::sample::{ Spec, Format };
use pulse::mainloop::threaded::Mainloop;
use pulse::volume::{ ChannelVolumes, Volume };
use pulse::context::{ Context, FlagSet as CtxFlagSet };
use pulse::stream::{ Stream, FlagSet as StreamFlagSet, PeekResult };
use pulse::context::subscribe::{ InterestMaskSet, Facility, Operation };
use pulse::context::introspect::{ SourceInfo, SinkInfo, SinkInputInfo, SourceOutputInfo };

use std::collections::HashMap;
use std::sync::mpsc::{ channel, Sender, Receiver };

use crate::shared::Shared;
use crate::meter::MeterData;


/** Represents a stream's underlying libpulse type. */
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StreamType {
	Sink, SinkInput, Source, SourceOutput
}

impl Default for StreamType {
	fn default() -> Self { StreamType::Sink }
}


/** The message types that can be passed through the channel from async callbacks. */
enum TxMessage {
	Update(StreamType, TxData),
	Remove(StreamType, u32),
	Peak(StreamType, u32, u32)
}


/** Transferrable information pretaining to a stream. */
#[derive(Debug)]
pub struct TxData {
	pub data: MeterData,
	pub monitor_index: u32,
}


/** Stored representation of a monitored stream. */
pub struct StreamData {
	pub data: MeterData,

	pub peak: u32,
	pub monitor_index: u32,
	pub monitor: Shared<Stream>
}


/** Container for mspc channel sender & receiver. */
struct Channel<T> { tx: Sender<T>, rx: Receiver<T> }


/**
 * The main controller for all libpulse interactions.
 * Handles peak monitoring, stream discovery, and meter information.
 * Stores data for all known streams, allowing public access.
 */

pub struct PulseController {
	mainloop: Shared<Mainloop>,
	context: Shared<Context>,
	channel: Channel<TxMessage>,

	pub sinks: HashMap<u32, StreamData>,
	pub sink_inputs: HashMap<u32, StreamData>,
	pub sources: HashMap<u32, StreamData>,
	pub source_outputs: HashMap<u32, StreamData>,
}

impl PulseController {


	/**
	 * Create a new pulse controller, configuring
	 * (but not connecting to) the libpulse api.
	 */

	pub fn new() -> Self {
		let mut proplist = Proplist::new().unwrap();
		proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "Myxer").unwrap();

		let mainloop = Shared::new(Mainloop::new().expect("Failed to initialize pulse mainloop."));

		let context = Shared::new(
			Context::new_with_proplist(&*mainloop.borrow(), "Myxer Context", &proplist)
			.expect("Failed to initialize pulse context."));

		let ( tx, rx ) = channel::<TxMessage>();

		PulseController {
			mainloop, context,
			channel: Channel { tx, rx },

			sinks: HashMap::new(),
			sink_inputs: HashMap::new(),
			sources: HashMap::new(),
			source_outputs: HashMap::new()
		}
	}


	/**
	 * Initiates a connection to pulse. Blocks until success, panics on failure.
	 * TODO: Graceful error handling, with debug message.
	 * TODO: Try to see if there's a way to avoid using unsafe? It's in the docs...  but...?
	 */

	pub fn connect(&mut self) {
		let mut mainloop = self.mainloop.borrow_mut();
		let mut ctx = self.context.borrow_mut();

		let mainloop_shr_ref = self.mainloop.clone();
		let ctx_shr_ref = self.context.clone();

		ctx.set_state_callback(Some(Box::new(move || {
			match unsafe { (*ctx_shr_ref.as_ptr()).get_state() } {
				pulse::context::State::Ready |
				pulse::context::State::Failed |
				pulse::context::State::Terminated =>
					unsafe { (*mainloop_shr_ref.as_ptr()).signal(false); },
				_ => {},
			}
		})));

		ctx.connect(None, CtxFlagSet::NOFLAGS, None)
			.expect("Failed to connect to the pulse server.");

		mainloop.lock();
		mainloop.start().expect("Failed to start pulse mainloop.");

		loop {
			match ctx.get_state() {
				pulse::context::State::Ready => {
					ctx.set_state_callback(None);
					mainloop.unlock();
					break;
				},
				pulse::context::State::Failed |
				pulse::context::State::Terminated => {
					eprintln!("Context state failed/terminated, quitting...");
					mainloop.unlock();
					mainloop.stop();
					panic!("Pulse session terminated.");
				},
				_ => { mainloop.wait(); },
			}
		}

		drop(ctx);
		drop(mainloop);
		self.subscribe();
	}


	/**
	 * Asychronously sets the volume of a stream to a raw integer value.
	 *
	 * @param {StreamType} t - The type of stream.
	 * @param {u32} index - The index of the stream.
	 * @param {u32} vol - The desired volume.
	 */

	pub fn set_volume(&self, t: StreamType, index: u32, vol: u32) {
		let channels = if t == StreamType::Sink || t == StreamType::SinkInput { 2 } else { 1 };
		let mut volumes = ChannelVolumes::default();
		let volume = Volume(vol);
	
		volumes.set_len(channels);
		volumes.set(channels, volume);

		let mut introspect = self.context.borrow().introspect();
		
		match t {
			StreamType::Sink => drop(introspect.set_sink_volume_by_index(index, &volumes, None)),
			StreamType::SinkInput => drop(introspect.set_sink_input_volume(index, &volumes, None)),
			StreamType::Source => drop(introspect.set_source_volume_by_index(index, &volumes, None)),
			StreamType::SourceOutput => drop(introspect.set_source_output_volume(index, &volumes, None))
		};
	}


	/**
	 * Asynchronously mutes or unmutes a stream.
	 *
	 * @param {StreamType} t - The type of stream.
	 * @param {u32} index - The index of the stream.
	 * @param {bool} muted - Whether the stream should be muted or not.
	 */

	pub fn set_muted(&self, t: StreamType, index: u32, muted: bool) {
		let mut introspect = self.context.borrow().introspect();
		match t {
			StreamType::Sink => drop(introspect.set_sink_mute_by_index(index, muted, None)),
			StreamType::SinkInput => drop(introspect.set_sink_input_mute(index, muted, None)),
			StreamType::Source => drop(introspect.set_source_mute_by_index(index, muted, None)),
			StreamType::SourceOutput => drop(introspect.set_source_output_mute(index, muted, None))
		};
	}


	/**
	 * Bind listeners to the required libpulse events, populate sink stores.
	 * Separated from connect() for readability.
	 */

	pub fn subscribe(&mut self) {
		fn tx_sink(tx: &Sender<TxMessage>, result: ListResult<&SinkInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::Update(StreamType::Sink, TxData {
					data: MeterData {
						t: StreamType::Sink,
						index: item.index,
						icon: None,
						name: item.description.clone().unwrap().into_owned(),
						volume: item.volume.avg().0,
						muted: item.mute
					},
					monitor_index: item.monitor_source
				})).unwrap();
			};
		};

		fn tx_sink_input(tx: &Sender<TxMessage>, result: ListResult<&SinkInputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::Update(StreamType::SinkInput, TxData {
					data: MeterData {
						t: StreamType::SinkInput,
						index: item.index,
						icon: Some(item.proplist.get_str("application.icon_name").unwrap_or("audio-card".to_owned())),
						name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
						volume: item.volume.avg().0,
						muted: item.mute
					},
					monitor_index: item.sink
				})).unwrap();
			};
		};

		fn tx_source(tx: &Sender<TxMessage>, result: ListResult<&SourceInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::Update(StreamType::Source, TxData {
					data: MeterData {
						t: StreamType::Source,
						index: item.index,
						icon: None,
						name: item.description.clone().unwrap().into_owned(),
						volume: item.volume.avg().0,
						muted: item.mute
					},
					monitor_index: item.index
				})).unwrap();
			};
		};

		fn tx_source_output(tx: &Sender<TxMessage>, result: ListResult<&SourceOutputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				let app_id = item.proplist.get_str("application.process.binary").unwrap_or("".to_owned());
				if app_id.contains("pavucontrol") || app_id.contains("myxer") { return; }
				tx.send(TxMessage::Update(StreamType::SourceOutput, TxData {
					data: MeterData {
						t: StreamType::SourceOutput,
						index: item.index,
						icon: Some(item.proplist.get_str("application.icon_name").unwrap_or("audio-card".to_owned())),
						name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
						volume: item.volume.avg().0,
						muted: item.mute
					},
					monitor_index: item.source
				})).unwrap();
			};
		};

		let mut context = self.context.borrow_mut();
		let introspect = context.introspect();

		let tx = self.channel.tx.clone();
		introspect.get_sink_info_list(move |res| tx_sink(&tx, res));
		let tx = self.channel.tx.clone();
		introspect.get_sink_input_info_list(move |res| tx_sink_input(&tx, res));
		let tx = self.channel.tx.clone();
		introspect.get_source_info_list(move |res| tx_source(&tx, res));
		let tx = self.channel.tx.clone();
		introspect.get_source_output_info_list(move |res| tx_source_output(&tx, res));
		
		let tx = self.channel.tx.clone();
		context.subscribe(InterestMaskSet::SINK | InterestMaskSet::SINK_INPUT |
			InterestMaskSet::SOURCE | InterestMaskSet::SOURCE_OUTPUT, |_|());
		context.set_subscribe_callback(Some(Box::new(move |fac, op, index| {
			let tx = tx.clone();
			let facility = fac.unwrap();
			let operation = op.unwrap();

			match facility {
				Facility::Sink => match operation {
					Operation::Removed => tx.send(TxMessage::Remove(StreamType::Sink, index)).unwrap(),
					_ => drop(introspect.get_sink_info_by_index(index, move |res| tx_sink(&tx, res)))
				},
				Facility::SinkInput => match operation {
					Operation::Removed => tx.send(TxMessage::Remove(StreamType::SinkInput, index)).unwrap(),
					_ => drop(introspect.get_sink_input_info(index, move |res| tx_sink_input(&tx, res)))
				},
				Facility::Source => match operation {
					Operation::Removed => tx.send(TxMessage::Remove(StreamType::Source, index)).unwrap(),
					_ => drop(introspect.get_source_info_by_index(index, move |res| tx_source(&tx, res)))
				},
				Facility::SourceOutput => match operation {
					Operation::Removed => tx.send(TxMessage::Remove(StreamType::SourceOutput, index)).unwrap(),
					_ => drop(introspect.get_source_output_info(index, move |res| tx_source_output(&tx, res)))
				},
				_ => ()
			};
		})));
	}


	/**
	 * Update the stored streams from the pending operations in the channel.
	 *
	 * @returns a value indicating if a visual update is required.
	 */

	pub fn update(&mut self) -> bool {
		let mut received = false;

		loop {
			let res = self.channel.rx.try_recv();
			match res {
				Ok(res) => {
					received = true;
					match res {
						TxMessage::Update(t, data) => self.update_stream(t, &data),
						TxMessage::Remove(t, ind) => self.remove_stream(t, ind),
						TxMessage::Peak(t, index, peak) => self.update_peak(t, index, peak),
					}
				},
				_ => break
			}
		}

		received
	}


	/**
	 * Closes the pulse connection and cleans up any dangling references.
	 * TODO: Close all streams here.
	 */

	pub fn cleanup(&mut self) {
		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.lock();
		mainloop.stop();
		mainloop.unlock();
	}


	/**
	 * Updates a stream in the store, or creates a new one and begins monitoring.
	 *
	 * @param {StreamType} t - The type of stream to update.
	 * @param {&TxData} stream - The stream's data.
	 */

	fn update_stream(&mut self, t: StreamType, stream: &TxData) {
		let data = stream.data.clone();
		let index = data.index;

		let entry = match t {
			StreamType::Sink => self.sinks.get_mut(&index),
			StreamType::SinkInput => self.sink_inputs.get_mut(&index),
			StreamType::Source => self.sources.get_mut(&index),
			StreamType::SourceOutput => self.source_outputs.get_mut(&index),
		};

		if let Some(stream) = entry { stream.data = data; }
		else {
			let source_str = stream.monitor_index.to_string();
			let monitor = self.create_monitor_stream(t, if t == StreamType::SinkInput { None } else { Some(source_str.as_str()) }, index);
			let data = StreamData { data, peak: 0, monitor, monitor_index: stream.monitor_index };
			match t {
				StreamType::Sink => self.sinks.insert(index, data),
				StreamType::SinkInput => self.sink_inputs.insert(index, data),
				StreamType::Source => self.sources.insert(index, data),
				StreamType::SourceOutput => self.source_outputs.insert(index, data)
			};
		}
	}


	/**
	 * Removes a stream from the store, stopping the monitor, if there is one.
	 *
	 * @param {StreamType} t - The type of stream to remove.
	 * @param {u32} index - The index of the stream to remove.
	 */

	fn remove_stream(&mut self, t: StreamType, index: u32) {
		let stream_opt = match t {
			StreamType::Sink => self.sinks.get_mut(&index),
			StreamType::SinkInput => self.sink_inputs.get_mut(&index),
			StreamType::Source => self.sources.get_mut(&index),
			StreamType::SourceOutput => self.source_outputs.get_mut(&index),
		};

		if let Some(stream) = stream_opt {
			let mut monitor = stream.monitor.borrow_mut();
			if monitor.get_state().is_good() {
				monitor.set_read_callback(None);
				monitor.disconnect().unwrap();
			}
		}

		match t {
			StreamType::Sink => self.sinks.remove(&index),
			StreamType::SinkInput => self.sink_inputs.remove(&index),
			StreamType::Source => self.sources.remove(&index),
			StreamType::SourceOutput => self.source_outputs.remove(&index),
		};
	}


	/**
	 * Updates a stored stream's peak value.
	 *
	 * @param {StreamType} t - The type of stream to update.
	 * @param {u32} index - The index of the stream to update.
	 * @param {u32} peak - The peak value to set.
	 */

	fn update_peak(&mut self, t: StreamType, index: u32, peak: u32) {
		match t {
			StreamType::Sink => self.sinks.entry(index).and_modify(|e| e.peak = peak),
			StreamType::SinkInput => self.sink_inputs.entry(index).and_modify(|e| e.peak = peak),
			StreamType::Source => self.sources.entry(index).and_modify(|e| e.peak = peak),
			StreamType::SourceOutput => self.source_outputs.entry(index).and_modify(|e| e.peak = peak)
		};
	}


	/**
	 * Creates a monitor stream for the stream specified, and returns it.
	 * Panics if there's an error. TODO: Don't panic.
	 *
	 * @param {StreamType} t - The type of stream to monitor.
	 * @param {Option<&str>} source - The source string of the stream, if there is one.
	 * @param {u32} stream_index - The index of the stream to monitor.
	 */

	fn create_monitor_stream(&mut self, t: StreamType, source: Option<&str>, stream_index: u32) -> Shared<Stream> {
		fn read_callback(stream: &mut Stream, t: StreamType, index: u32, tx: &Sender<TxMessage>) {
			let mut raw_peak = 0.0;
			while stream.readable_size().is_some() {
				match stream.peek().unwrap() {
					PeekResult::Hole(_) => stream.discard().unwrap(),
					PeekResult::Data(b) => {
						let buf = slice_as_array!(b, [u8; 4]).expect("Bad length.");
						raw_peak = f32::from_le_bytes(*buf).max(raw_peak);
						stream.discard().unwrap();
					},
					_ => break
				}
			}
			let peak = (raw_peak.sqrt() * 65535.0 * 1.5).round() as u32;
			tx.send(TxMessage::Peak(t, index, peak)).unwrap();
		}

		let mut attr = BufferAttr::default();
		attr.fragsize = 4;
		attr.maxlength = u32::MAX;
		
		let spec = Spec { channels: 1, format: Format::F32le, rate: 30 };
		assert!(spec.is_valid());
		
		let s = Shared::new(Stream::new(&mut self.context.borrow_mut(), "Peak Detect", &spec, None).unwrap());
		{
			let mut stream = s.borrow_mut();
			if t == StreamType::SinkInput {
				stream.set_monitor_stream(stream_index).unwrap();
			}

			stream.connect_record(source, Some(&attr),
				StreamFlagSet::DONT_MOVE | StreamFlagSet::ADJUST_LATENCY | StreamFlagSet::PEAK_DETECT).unwrap();

			let t = t.clone();
			let sc = s.clone();
			let txc = self.channel.tx.clone();
			stream.set_read_callback(Some(Box::new(move |_| read_callback(&mut sc.borrow_mut(), t, stream_index, &txc))));
			// let sc = s.clone();
			// stream.set_state_callback(Some(Box::new(move || println!("{:?}", sc.borrow_mut().get_state()))));
		}

		return s;
	}
}

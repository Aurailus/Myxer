/*!
 * The main data-store / binding module that interacts with the pulse audio server.
 * Monitors the pulse server for updates, and also exposes methods to request changes.
 */

use slice_as_array::{ slice_as_array, slice_as_array_transmute };

use libpulse::def::BufferAttr;
use libpulse::callbacks::ListResult;
use libpulse::volume::ChannelVolumes;
use libpulse::sample::{ Spec, Format };
use libpulse::mainloop::threaded::Mainloop;
use libpulse::proplist::{ Proplist, properties };
use libpulse::stream::{ Stream, FlagSet as StreamFlagSet, PeekResult };
use libpulse::context::subscribe::{ InterestMaskSet, Facility, Operation };
use libpulse::context::{ Context, FlagSet as CtxFlagSet, State as ContextState };
use libpulse::context::introspect::{ ServerInfo, SourceInfo, SinkInfo, SinkInputInfo, SourceOutputInfo, CardInfo };

use std::collections::HashMap;
use std::sync::mpsc::{ channel, Sender, Receiver };

use super::shared::Shared;
use super::card::CardData;
use super::meter::MeterData;


/**
 * Represents a stream's underlying type.
 */

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StreamType {
	Sink, SinkInput, Source, SourceOutput
}

impl Default for StreamType {
	fn default() -> Self { StreamType::Sink }
}


/**
 * The different message types that can be passed from the pulse
 * thread to the data store. They contain data related to the
 * current state of the pulse server.
 */

enum TxMessage {
	Default(String, String),
	StreamUpdate(StreamType, TxStreamData),
	StreamRemove(StreamType, u32),
	CardUpdate(CardData),
	CardRemove(u32),
	Peak(StreamType, u32, u32)
}


/**
 * Transferrable information pretaining to a stream.
 */

#[derive(Debug)]
pub struct TxStreamData {
	pub data: MeterData,
	pub monitor_index: u32,
}


/**
 * Stored representation of a pulse stream.
 * The stream index is not in this struct, but
 * it is the index it is keyed under in its hashmap.
 */

pub struct StreamData {
	pub data: MeterData,

	pub peak: u32,
	pub monitor_index: u32,
	pub monitor: Shared<Stream>
}


/** Container for mspc channel sender & receiver. */
struct Channel<T> { tx: Sender<T>, rx: Receiver<T> }


/**
 * The main controller for all pulse server interactions.
 * Handles peak monitoring, stream discovery, and meter information.
 * Stores data for all known streams, allowing public access.
 */

pub struct Pulse {
	mainloop: Shared<Mainloop>,
	context: Shared<Context>,
	channel: Channel<TxMessage>,

	pub default_sink: u32,
	pub default_source: u32,
	pub active_sink: u32,
	pub active_source: u32,

	pub sinks: HashMap<u32, StreamData>,
	pub sink_inputs: HashMap<u32, StreamData>,
	pub sources: HashMap<u32, StreamData>,
	pub source_outputs: HashMap<u32, StreamData>,
	pub cards: HashMap<u32, CardData>,
}

impl Pulse {

	/**
	 * Creates a new pulse controller, configuring (but not initializing) the pulse connection.
	 */

	pub fn new() -> Self {
		let mut proplist = Proplist::new().unwrap();
		proplist.set_str(properties::APPLICATION_NAME, "Myxer").unwrap();

		let mainloop = Shared::new(Mainloop::new().expect("Failed to initialize pulse mainloop."));

		let context = Shared::new(
			Context::new_with_proplist(&*mainloop.borrow(), "Myxer Context", &proplist)
			.expect("Failed to initialize pulse context."));

		let ( tx, rx ) = channel::<TxMessage>();

		Pulse {
			mainloop, context,
			channel: Channel { tx, rx },

			default_sink: u32::MAX,
			default_source: u32::MAX,
			active_sink: u32::MAX,
			active_source: u32::MAX,

			sinks: HashMap::new(),
			sink_inputs: HashMap::new(),
			sources: HashMap::new(),
			source_outputs: HashMap::new(),
			cards: HashMap::new()
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
				ContextState::Ready |
				ContextState::Failed |
				ContextState::Terminated =>
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
				ContextState::Ready => {
					ctx.set_state_callback(None);
					mainloop.unlock();
					break;
				},
				ContextState::Failed |
				ContextState::Terminated => {
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
	 * Asynchronously sets the default sink to the index provided.
	 * This is sometimes described as the fallback device.
	 *
	 * * `sink` - The sink index to set as the default.
	 */

	pub fn set_default_sink(&self, sink: u32) {
		if let Some(sink) = self.sinks.get(&sink) {
			let mut mainloop = self.mainloop.borrow_mut();
			mainloop.lock();
			self.context.borrow_mut().set_default_sink(&sink.data.name, |_|());
			mainloop.unlock();
		}
	}


	/**
	 * Asynchronously sets the default source to the index provided.
	 * This is sometimes described as the fallback device.
	 *
	 * * `source` - The source index to set as the default.
	 */

	pub fn set_default_source(&self, source: u32) {
		if let Some(source) = self.sources.get(&source) {
			let mut mainloop = self.mainloop.borrow_mut();
			mainloop.lock();
			self.context.borrow_mut().set_default_source(&source.data.name, |_|());
			mainloop.unlock();
		}
	}


	/**
	 * Sets the 'active' sink to the index provided.
	 * This is the sink that is currently displayed on the interface.
	 *
	 * * `sink` - The sink index to set as active.
	 */

	pub fn set_active_sink(&mut self, sink: u32) {
		self.active_sink = sink;
	}


	/**
	 * Sets the 'active' source to the index provided.
	 * This is the source that is currently displayed on the interface.
	 *
	 * * `source` - The source to set as active.
	 */

	pub fn set_active_source(&mut self, source: u32) {
		self.active_source = source;
	}


	/**
	 * Sets the volume of the stream to the volumes specified.
	 * This operation is asynchronous, so changes will not be reflected immediately.
	 *
	 * * `t`       - The type of stream to set the volume of.
	 * * `index`   - The index of the stream to set the volume of.
	 * * `volumes` - The desired volumes to set the channels of the stream to.
	 */

	pub fn set_volume(&self, t: StreamType, index: u32, volumes: ChannelVolumes) {
		let mut introspect = self.context.borrow().introspect();
		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.lock();

		match t {
			StreamType::Sink => drop(introspect.set_sink_volume_by_index(index, &volumes, None)),
			StreamType::SinkInput => drop(introspect.set_sink_input_volume(index, &volumes, None)),
			StreamType::Source => drop(introspect.set_source_volume_by_index(index, &volumes, None)),
			StreamType::SourceOutput => drop(introspect.set_source_output_volume(index, &volumes, None))
		};

		mainloop.unlock();
	}


	/**
	 * Mutes or unmutes a stream.
	 * This operation is asynchronous, so changes will not be reflected immediately.
	 *
	 * * `t`     - The type of stream to update.
	 * * `index` - The index of the stream to update.
	 * * `mute`  - Whether the stream should be muted or not.
	 */

	pub fn set_muted(&self, t: StreamType, index: u32, mute: bool) {
		let mut introspect = self.context.borrow().introspect();
		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.lock();

		match t {
			StreamType::Sink => drop(introspect.set_sink_mute_by_index(index, mute, None)),
			StreamType::SinkInput => drop(introspect.set_sink_input_mute(index, mute, None)),
			StreamType::Source => drop(introspect.set_source_mute_by_index(index, mute, None)),
			StreamType::SourceOutput => drop(introspect.set_source_output_mute(index, mute, None))
		};

		mainloop.unlock();
	}


	/**
	 * Set's a sound card's profile.
	 * This effects how the card behaves, and how the system can utilize it.
	 *
	 * * `index`   - The card index to update.
	 * * `profile` - The profile name to update the card to.
	 */
	 
	pub fn set_card_profile(&self, index: u32, profile: &str) {
		let mut introspect = self.context.borrow().introspect();
		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.lock();
		introspect.set_card_profile_by_index(index, profile, None);
		mainloop.unlock();
	}


	/**
	 * Binds listeners to server events, and triggers an
	 * initial sweep to populate the internal stores.
	 * Called by connect(), separated for readability.
	 */

	fn subscribe(&mut self) {
		/** Updates the client when the server information changes. */
		fn tx_server(tx: &Sender<TxMessage>, item: &ServerInfo<'_>) {
			tx.send(TxMessage::Default(item.default_sink_name.clone().unwrap().into_owned(),
				item.default_source_name.clone().unwrap().into_owned())).unwrap();
		};

		/** Updates the client when a sink changes. */
		fn tx_sink(tx: &Sender<TxMessage>, result: ListResult<&SinkInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::StreamUpdate(StreamType::Sink, TxStreamData {
					data: MeterData {
						t: StreamType::Sink,
						index: item.index,
						icon: "multimedia-volume-control".to_owned(),
						name: item.name.clone().unwrap().into_owned(),
						description: item.description.clone().unwrap().into_owned(),
						volume: item.volume,
						muted: item.mute
					},
					monitor_index: item.monitor_source
				})).unwrap();
			};
		};

		/** Updates the client when a sink input changes. */
		fn tx_sink_input(tx: &Sender<TxMessage>, result: ListResult<&SinkInputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::StreamUpdate(StreamType::SinkInput, TxStreamData {
					data: MeterData {
						t: StreamType::SinkInput,
						index: item.index,
						icon: item.proplist.get_str("application.icon_name").unwrap_or_else(|| "audio-card".to_owned()),
						name: item.name.clone().unwrap().into_owned(),
						description: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
						volume: item.volume,
						muted: item.mute
					},
					monitor_index: item.sink
				})).unwrap();
			};
		};

		/** Updates the client when a source changes. */
		fn tx_source(tx: &Sender<TxMessage>, result: ListResult<&SourceInfo<'_>>) {
			if let ListResult::Item(item) = result {
				let name = item.name.clone().unwrap().into_owned();
				if name.ends_with(".monitor") { return; }
				tx.send(TxMessage::StreamUpdate(StreamType::Source, TxStreamData {
					data: MeterData {
						t: StreamType::Source,
						index: item.index,
						icon: "audio-input-microphone".to_owned(),
						name: item.name.clone().unwrap().into_owned(),
						description: item.description.clone().unwrap().into_owned(),
						volume: item.volume,
						muted: item.mute
					},
					monitor_index: item.index
				})).unwrap();
			};
		};

		/** Updates the client when a source output changes. */
		fn tx_source_output(tx: &Sender<TxMessage>, result: ListResult<&SourceOutputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				let app_id = item.proplist.get_str("application.process.binary").unwrap_or("".to_owned()).to_lowercase();
				if app_id.contains("pavucontrol") || app_id.contains("myxer") { return; }
				tx.send(TxMessage::StreamUpdate(StreamType::SourceOutput, TxStreamData {
					data: MeterData {
						t: StreamType::SourceOutput,
						index: item.index,
						icon: item.proplist.get_str("application.icon_name").unwrap_or_else(|| "audio-card".to_owned()),
						name: item.name.clone().unwrap().into_owned(),
						description: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
						volume: item.volume,
						muted: item.mute
					},
					monitor_index: item.source
				})).unwrap();
			};
		};

		/** Updates the client when a sound card changes. */
		fn tx_card(tx: &Sender<TxMessage>, result: ListResult<&CardInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::CardUpdate(CardData {
					index: item.index,
					name: item.proplist.get_str("device.description").unwrap_or("".to_owned()),
					icon: item.proplist.get_str("device.icon_name").unwrap_or_else(|| "audio-card-pci".to_owned()),
					profiles: item.profiles.iter().map(|p| (p.name.as_ref().unwrap().clone().into_owned(),
						p.description.as_ref().unwrap().clone().into_owned())).collect(),
					active_profile: item.active_profile.as_ref().unwrap().name.as_ref().unwrap().clone().into_owned()
				})).unwrap();
			}
		}

		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.lock();
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
		introspect.get_card_info_list(move |res| tx_card(&tx, res));
		let tx = self.channel.tx.clone();
		introspect.get_server_info(move |res| tx_server(&tx, res));
		
		let tx = self.channel.tx.clone();
		context.subscribe(InterestMaskSet::SERVER | InterestMaskSet::SINK | InterestMaskSet::SINK_INPUT |
			InterestMaskSet::SOURCE | InterestMaskSet::SOURCE_OUTPUT | InterestMaskSet::CARD, |_|());
		context.set_subscribe_callback(Some(Box::new(move |fac, op, index| {
			let tx = tx.clone();
			let facility = fac.unwrap();
			let operation = op.unwrap();

			match facility {
				Facility::Server => drop(introspect.get_server_info(move |res| tx_server(&tx, res))),
				Facility::Sink => match operation {
					Operation::Removed => tx.send(TxMessage::StreamRemove(StreamType::Sink, index)).unwrap(),
					_ => drop(introspect.get_sink_info_by_index(index, move |res| tx_sink(&tx, res)))
				},
				Facility::SinkInput => match operation {
					Operation::Removed => tx.send(TxMessage::StreamRemove(StreamType::SinkInput, index)).unwrap(),
					_ => drop(introspect.get_sink_input_info(index, move |res| tx_sink_input(&tx, res)))
				},
				Facility::Source => match operation {
					Operation::Removed => tx.send(TxMessage::StreamRemove(StreamType::Source, index)).unwrap(),
					_ => drop(introspect.get_source_info_by_index(index, move |res| tx_source(&tx, res)))
				},
				Facility::SourceOutput => match operation {
					Operation::Removed => tx.send(TxMessage::StreamRemove(StreamType::SourceOutput, index)).unwrap(),
					_ => drop(introspect.get_source_output_info(index, move |res| tx_source_output(&tx, res)))
				},
				Facility::Card => match operation {
					Operation::Removed => tx.send(TxMessage::CardRemove(index)).unwrap(),
					_ => drop(introspect.get_card_info_by_index(index, move |res| tx_card(&tx, res)))
				},
				_ => ()
			};
		})));

		mainloop.unlock();
	}


	/**
	 * Handles queued messages from the pulse thread, updating the internal storage.
	 * Returns a boolean indicating that a layout refresh is required.
	 */

	pub fn update(&mut self) -> bool {
		let mut received = false;

		loop {
			let res = self.channel.rx.try_recv();
			match res {
				Ok(res) => {
					received = true;
					match res {
						TxMessage::Default(sink, source) => self.update_default(sink, source),
						TxMessage::StreamUpdate(t, data) => self.update_stream(t, &data),
						TxMessage::StreamRemove(t, ind) => self.remove_stream(t, ind),
						TxMessage::CardUpdate(data) => self.update_card(&data),
						TxMessage::CardRemove(ind) => self.remove_card(ind),
						TxMessage::Peak(t, ind, peak) => self.update_peak(t, ind, peak),
					}
				},
				_ => break
			}
		}

		received
	}


	/**
	 * Closes the connection to the pulse server, and cleans up any dangling monitors.
	 * After this operation, no other methods should be called, and the instance should be freed from memory.
	 */

	pub fn cleanup(&mut self) {
		while let Some((i, _)) = self.sinks.iter().next() { let i = *i; self.remove_stream(StreamType::Sink, i) }
		while let Some((i, _)) = self.sink_inputs.iter().next() { let i = *i; self.remove_stream(StreamType::SinkInput, i) }
		while let Some((i, _)) = self.sources.iter().next() { let i = *i; self.remove_stream(StreamType::Source, i) }
		while let Some((i, _)) = self.source_outputs.iter().next() { let i = *i; self.remove_stream(StreamType::SourceOutput, i) }
		
		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.stop();
	}


	/**
	 * Updates the stored default sink and source to the ones identified.
	 * This method is called by the update method, the names are provided by the pulse server.
	 *
	 * * `sink`   - The default sink.
	 * * `source` - The default source.
	 */

	fn update_default(&mut self, sink: String, source: String) {
		for (i, v) in &self.sinks {
			if v.data.name == sink {
				self.default_sink = *i;
				self.active_sink = *i;
				break;
			}
		}

		for (i, v) in &self.sources {
			if v.data.name == source {
				self.default_source = *i;
				self.active_source = *i;
				break;
			}
		}
	}


	/**
	 * Updates a stream in the store, or creates a new one and begins monitoring the peaks.
	 * This method is called by the update method, the data is provided by the pulse server.
	 *
	 * * `t`      - The type of stream to update.
	 * * `stream` - The new stream's data.
	 */

	fn update_stream(&mut self, t: StreamType, stream: &TxStreamData) {
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
			let monitor = self.create_monitor_stream(t, if t == StreamType::SinkInput { None } else { Some(&source_str) }, index);
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
	 * This method is called by the update method, the data is provided by the pulse server.
	 *
	 * * `t`     - The type of stream to remove.
	 * * `index` - The index of the stream to remove.
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
			let mut mainloop = self.mainloop.borrow_mut();
			mainloop.lock();
			if monitor.get_state().is_good() {
				monitor.set_read_callback(None);
				let _ = monitor.disconnect();
			}
			mainloop.unlock();
		}

		match t {
			StreamType::Sink => self.sinks.remove(&index),
			StreamType::SinkInput => self.sink_inputs.remove(&index),
			StreamType::Source => self.sources.remove(&index),
			StreamType::SourceOutput => self.source_outputs.remove(&index),
		};
	}


	/**
	 * Updates a stored stream's peak.
	 * This method is called by the update method, the data is provided by a monitor stream.
	 *
	 * * `t`     - The type of stream to update.
	 * * `index` - The index of the stream to update.
	 * * `peak`  - The peak value to store.
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
	 * Panics if there's an error.
	 * TODO: Don't panic.
	 *
	 * * `t`            - The type of stream to monitor.
	 * * `source`       - The source string of the stream, if one is needed.
	 * * `stream_index` - The index of the stream to monitor.
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

			let mut mainloop = self.mainloop.borrow_mut();
			mainloop.lock();
			stream.connect_record(source, Some(&attr),
				StreamFlagSet::DONT_MOVE | StreamFlagSet::ADJUST_LATENCY | StreamFlagSet::PEAK_DETECT).unwrap();
			mainloop.unlock();

			let t = t.clone();
			let sc = s.clone();
			let txc = self.channel.tx.clone();
			stream.set_read_callback(Some(Box::new(move |_| read_callback(&mut sc.borrow_mut(), t, stream_index, &txc))));
		}

		return s;
	}


	/**
	 * Updates a card in the store, or creates a new one.
	 * This method is called by the update method, the data is provided by the pulse server.
	 *
	 * * `data` - The card's data.
	 */

	fn update_card(&mut self, data: &CardData) {
		let index = data.index;
		self.cards.insert(index, data.clone());
	}


	/**
	 * Removes a card from the store.
	 * This method is called by the update method, the data is provided by the pulse server.
	 *
	 * * `index` - The index of the stream to remove.
	 */

	fn remove_card(&mut self, index: u32) {
		self.cards.remove(&index);
	}
}

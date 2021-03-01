use std::collections::HashMap;
use std::sync::mpsc::{ channel, Sender, Receiver };

use crate::shared::Shared;
use crate::pulse_data::{ Sink, SinkData, SinkInput, SinkInputData, Source, SourceData, SourceOutput, SourceOutputData };

use pulse::mainloop::threaded::Mainloop;
use pulse::context::{ Context, FlagSet as CtxFlagSet };

use pulse::proplist::Proplist;
use pulse::def::{ BufferAttr };
use pulse::callbacks::ListResult;
use pulse::sample::{ Spec, Format };
use pulse::volume::{ ChannelVolumes, Volume };
use pulse::stream::{ Stream, FlagSet as StreamFlagSet, PeekResult };
use pulse::context::subscribe::{ InterestMaskSet, Facility, Operation };
use pulse::context::introspect::{ SourceInfo, SinkInfo, SinkInputInfo, SourceOutputInfo };

#[derive(Copy)]
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
enum PeakType {
	Sink, SinkInput, Source, SourceOutput
}

enum TxMessage {
	SinkUpdate(SinkData),
	SinkRemove(u32),
	SinkInputUpdate(SinkInputData),
	SinkInputRemove(u32),
	SourceUpdate(SourceData),
	SourceRemove(u32),
	SourceOutputUpdate(SourceOutputData),
	SourceOutputRemove(u32),
	Peak(PeakType, u32, u32)
}

struct Channel<T> {
	tx: Sender<T>,
	rx: Receiver<T>
}

pub struct PulseController {
	mainloop: Shared<Mainloop>,
	context: Shared<Context>,
	channel: Channel<TxMessage>,

	pub sinks: HashMap<u32, Sink>,
	pub sink_inputs: HashMap<u32, SinkInput>,
	pub sources: HashMap<u32, Source>,
	pub source_outputs: HashMap<u32, SourceOutput>,
}

impl PulseController {
	pub fn new() -> Self {
		let mut proplist = Proplist::new().unwrap();
		proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "Myxer")
			.expect("PulseController: Failed to set application name.");

		let mainloop = Shared::new(Mainloop::new()
			.expect("PulseController: Failed to initialize mainloop."));

		let context = Shared::new(
			Context::new_with_proplist(&*mainloop.borrow(), "Myxer Context", &proplist)
			.expect("PulseController: Failed to initialize context."));

		let ( tx, rx ) = channel::<TxMessage>();

		PulseController {
			mainloop: mainloop,
			context: context,
			channel: Channel { tx, rx },

			sinks: HashMap::new(),
			sink_inputs: HashMap::new(),
			sources: HashMap::new(),
			source_outputs: HashMap::new()
		}
	}

	pub fn connect(&mut self) {
		let mut mainloop = self.mainloop.borrow_mut();
		let mut ctx = self.context.borrow_mut();

		let mainloop_shr_ref = self.mainloop.clone();
		let ctx_shr_ref = self.context.clone();

		ctx.set_state_callback(Some(Box::new(move || {
			match unsafe { (*ctx_shr_ref.as_ptr()).get_state() } {
				pulse::context::State::Ready |
				pulse::context::State::Failed |
				pulse::context::State::Terminated => {
					unsafe { (*mainloop_shr_ref.as_ptr()).signal(false); }
				},
				_ => {},
			}
		})));

		ctx.connect(None, CtxFlagSet::NOFLAGS, None)
			.expect("PulseController: Failed to connect the context to server.");

		mainloop.lock();
		mainloop.start().expect("PulseController: Failed to start mainloop.");

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
	}

	pub fn set_sink_input_volume(&self, index: u32, vol: u32) {
		let mut volumes = ChannelVolumes::default();
		let volume = Volume(vol);
		volumes.set_len(2);
		volumes.set(2, volume);
		self.context.borrow().introspect().set_sink_input_volume(index, &volumes, None);
	}

	pub fn set_sink_input_muted(&self, index: u32, muted: bool) {
		self.context.borrow().introspect().set_sink_input_mute(index, muted, None);
	}

	pub fn subscribe(&mut self) {
		fn tx_sink(tx: &Sender<TxMessage>, result: ListResult<&SinkInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::SinkUpdate(SinkData {
					index: item.index,
					name: item.name.clone().unwrap().into_owned(),
					description: item.description.clone().unwrap().into_owned(),
					port_description: item.active_port.as_ref().unwrap().description.clone().unwrap().into_owned(),
					monitor_index: item.monitor_source,
					volume: item.volume.avg().0, muted: item.mute
				})).unwrap();
			};
		};

		fn tx_sink_input(tx: &Sender<TxMessage>, result: ListResult<&SinkInputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::SinkInputUpdate(SinkInputData {
					index: item.index, sink: item.sink,
					name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
					icon: item.proplist.get_str("application.icon_name").unwrap_or("audio-card".to_owned()),
					volume: item.volume.avg().0, muted: item.mute
				})).unwrap();
			};
		};

		fn tx_source(tx: &Sender<TxMessage>, result: ListResult<&SourceInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::SourceUpdate(SourceData {
					index: item.index,
					name: item.name.clone().unwrap().into_owned(),
					description: item.description.clone().unwrap().into_owned(),
					volume: item.volume.avg().0, muted: item.mute
				})).unwrap();
			};
		};

		fn tx_source_output(tx: &Sender<TxMessage>, result: ListResult<&SourceOutputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				let app_id = item.proplist.get_str("application.process.binary").unwrap_or("".to_owned());
				if app_id.contains("pavucontrol") || app_id.contains("myxer") { return; }
				tx.send(TxMessage::SourceOutputUpdate(SourceOutputData {
					index: item.index, source: item.source,
					name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
					icon: item.proplist.get_str("application.icon_name").unwrap_or("audio-card".to_owned()),
					volume: item.volume.avg().0, muted: item.mute
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
					Operation::Removed => tx.send(TxMessage::SinkRemove(index)).unwrap(),
					_ => { introspect.get_sink_info_by_index(index, move |res| tx_sink(&tx, res)); }
				},
				Facility::SinkInput => match operation {
					Operation::Removed => tx.send(TxMessage::SinkInputRemove(index)).unwrap(),
					_ => { introspect.get_sink_input_info(index, move |res| tx_sink_input(&tx, res)); }
				},
				Facility::Source => match operation {
					Operation::Removed => tx.send(TxMessage::SourceRemove(index)).unwrap(),
					_ => { introspect.get_source_info_by_index(index, move |res| tx_source(&tx, res)); }
				},
				Facility::SourceOutput => match operation {
					Operation::Removed => tx.send(TxMessage::SourceOutputRemove(index)).unwrap(),
					_ => { introspect.get_source_output_info(index, move |res| tx_source_output(&tx, res)); }
				},
				_ => ()
			};
		})));
	}

	pub fn update(&mut self) -> bool {
		let mut received = false;

		loop {
			let res = self.channel.rx.try_recv();
			match res {
				Ok(res) => {
					received = true;
					match res {
						TxMessage::SinkUpdate(sink) => self.sink_updated(sink),
						TxMessage::SinkRemove(sink) => self.sink_removed(sink),

						TxMessage::SinkInputUpdate(input) => self.sink_input_updated(input),
						TxMessage::SinkInputRemove(input) => self.sink_input_removed(input),

						TxMessage::SourceUpdate(source) => self.source_updated(source),
						TxMessage::SourceRemove(source) => self.source_removed(source),
						
						TxMessage::SourceOutputUpdate(output) => self.source_output_updated(output),
						TxMessage::SourceOutputRemove(output) => self.source_output_removed(output),

						TxMessage::Peak(t, index, peak) => self.update_peak(t, index, peak),
					}
				},
				_ => break
			}
		}

		received
	}

	pub fn cleanup(&mut self) {
		let mut mainloop = self.mainloop.borrow_mut();
		mainloop.lock();
		mainloop.stop();
		mainloop.unlock();
	}

	fn update_peak(&mut self, t: PeakType, index: u32, peak: u32) {
		match t {
			PeakType::Sink => self.sinks.get_mut(&index).unwrap().peak = peak,
			PeakType::SinkInput => self.sink_inputs.get_mut(&index).unwrap().peak = peak,
			PeakType::Source => self.sources.get_mut(&index).unwrap().peak = peak,
			PeakType::SourceOutput => self.source_outputs.get_mut(&index).unwrap().peak = peak
		}
	}

	fn sink_updated(&mut self, data: SinkData) {
		let index = data.index;
		let entry = self.sinks.get_mut(&index);
		if entry.is_some() { entry.unwrap().data = data; }
		else {
			let stream = self.create_monitor_stream(PeakType::Sink, Some(data.monitor_index.to_string().as_str()), index);
			self.sinks.insert(index, Sink { data, peak: 0, monitor: stream });
		}
	}

	fn sink_removed(&mut self, index: u32) {
		self.sinks.remove(&index);
	}

	fn sink_input_updated(&mut self, data: SinkInputData) {
		let index = data.index;
		let entry = self.sink_inputs.get_mut(&index);
		if entry.is_some() { entry.unwrap().data = data; }
		else {
			let stream = self.create_monitor_stream(PeakType::SinkInput, None, index);
			self.sink_inputs.insert(index, SinkInput { data, peak: 0, monitor: stream });
		}
	}

	fn sink_input_removed(&mut self, index: u32) {
		self.sink_inputs.remove(&index);
	}

	fn source_updated(&mut self, data: SourceData) {
		let index = data.index;
		let entry = self.sources.get_mut(&index);
		if entry.is_some() { entry.unwrap().data = data; }
		else {
			let stream = self.create_monitor_stream(PeakType::Source, Some(data.index.to_string().as_str()), index);
			self.sources.insert(index, Source { data, peak: 0, monitor: stream });
		}
	}

	fn source_removed(&mut self, index: u32) {
		self.sources.remove(&index);
	}

	fn source_output_updated(&mut self, data: SourceOutputData) {
		let index = data.index;
		let source = data.source;
		let entry = self.source_outputs.get_mut(&index);
		if entry.is_some() { entry.unwrap().data = data; }
		else {
			let stream = self.create_monitor_stream(PeakType::SourceOutput, Some(source.to_string().as_str()), index);
			self.source_outputs.insert(index, SourceOutput { data, peak: 0, monitor: stream });
		}
	}

	fn source_output_removed(&mut self, index: u32) {
		self.source_outputs.remove(&index);
	}

	fn create_monitor_stream(&mut self, t: PeakType, source: Option<&str>, stream_index: u32) -> Shared<Stream> {
		fn read_callback(stream: &mut Stream, t: PeakType, index: u32, tx: &Sender<TxMessage>) {
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
			if t == PeakType::SinkInput {
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

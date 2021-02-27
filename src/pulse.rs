use std::collections::HashMap;
use std::sync::mpsc::{ channel, Sender, Receiver };

use crate::shared::Shared;

use pulse::mainloop::threaded::Mainloop;
use pulse::context::{ Context, FlagSet as CtxFlagSet };

use pulse::proplist::Proplist;
use pulse::def::{ BufferAttr };
use pulse::callbacks::ListResult;
use pulse::sample::{ Spec, Format };
use pulse::stream::{ Stream, FlagSet as StreamFlagSet, PeekResult };
use pulse::context::introspect::{ SinkInputInfo, SinkInfo };
use pulse::context::subscribe::{ InterestMaskSet, Facility, Operation };

pub enum TxMessage {
	SinkUpdate(SinkData),
	SinkRemove(u32),

	SinkInputUpdate(SinkInputData),
	SinkInputRemove(u32)
}

#[derive(Debug)]
#[derive(Clone)]
pub struct SinkInputData {
	pub index: u32,
	pub sink: u32,
	pub name: String,
	pub icon: String,
	pub muted: bool,
	pub volume: pulse::volume::Volume,
}

pub struct SinkInput {
	pub data: SinkInputData,

	pub peak: u32,
	pub monitor: Shared<Stream>
}

#[derive(Debug)]
pub struct SinkData {
	pub index: u32,
	pub name: String,
	pub port_name: String,
	pub muted: bool,
	pub volume: pulse::volume::Volume
}

pub struct Sink {
	pub data: SinkData,

	pub peak: u32,
	pub monitor: Shared<Stream>
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
}

impl PulseController {
	pub fn new() -> Self {
		let mut proplist = Proplist::new().unwrap();
		proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "VMix")
			.expect("PulseController: Failed to set application name.");

		let mainloop = Shared::new(Mainloop::new()
			.expect("PulseController: Failed to initialize mainloop."));

		let context = Shared::new(
			Context::new_with_proplist(&*mainloop.borrow(), "VMix Context", &proplist)
			.expect("PulseController: Failed to initialize context."));

		let ( tx, rx ) = channel::<TxMessage>();

		PulseController {
			mainloop: mainloop,
			context: context,
			channel: Channel { tx, rx },

			sinks: HashMap::new(),
			sink_inputs: HashMap::new()
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

	pub fn subscribe(&mut self) {
		fn tx_sink(tx: &Sender<TxMessage>, result: ListResult<&SinkInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::SinkUpdate(SinkData {
					index: item.index,
					name: item.description.clone().unwrap().into_owned(),
					port_name: item.active_port.as_ref().unwrap().description.clone().unwrap().into_owned(),
					volume: item.volume.avg(), muted: item.mute
				})).unwrap();
			};
		};

		fn tx_sink_input(tx: &Sender<TxMessage>, result: ListResult<&SinkInputInfo<'_>>) {
			if let ListResult::Item(item) = result {
				tx.send(TxMessage::SinkInputUpdate(SinkInputData {
					index: item.index, sink: item.sink,
					name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
					icon: item.proplist.get_str("application.icon_name").unwrap_or("audio-card".to_owned()),
					volume: item.volume.avg(), muted: item.mute
				})).unwrap();
			};
		};

		let mut context = self.context.borrow_mut();
		let introspect = context.introspect();

		let tx = self.channel.tx.clone();
		introspect.get_sink_input_info_list(move |res| tx_sink_input(&tx, res));
		let tx = self.channel.tx.clone();
		introspect.get_sink_info_list(move |res| tx_sink(&tx, res));

		// introspect.get_client_info_list(|r| print!("{:?}", r));
		
		let tx = self.channel.tx.clone();
		context.subscribe(InterestMaskSet::SINK_INPUT | InterestMaskSet::SINK, |_|());
		context.set_subscribe_callback(Some(Box::new(move |fac, op, index| {
			let tx = tx.clone();
			let facility = fac.unwrap();
			let operation = op.unwrap();

			match facility {
				Facility::SinkInput => match operation {
					Operation::Removed => tx.send(TxMessage::SinkInputRemove(index)).unwrap(),
					_ => { introspect.get_sink_input_info(index, move |res| tx_sink_input(&tx, res)); }
				},
				Facility::Sink => match operation {
					Operation::Removed => tx.send(TxMessage::SinkRemove(index)).unwrap(),
					_ => { introspect.get_sink_info_by_index(index, move |res| tx_sink(&tx, res)); }
				},
				_ => ()
			};
		})));
	}

	pub fn update(&mut self) -> bool {
		// let mut received = false;

		loop {
			let res = self.channel.rx.try_recv();
			match res {
				Ok(res) => {
					// received = true;
					// println!("{:?}", res);
					match res {
						TxMessage::SinkUpdate(sink) => self.sink_updated(sink),
						TxMessage::SinkRemove(sink) => self.sink_removed(sink),
						// TxMessage::SinkPeak(index, peak) => println!("Sink Peak {}: {}", index, peak),

						TxMessage::SinkInputUpdate(input) => self.sink_input_updated(input),
						TxMessage::SinkInputRemove(input) => self.sink_input_removed(input),
						// TxMessage::SinkInputPeak(index, peak) => println!("Sink Peak {}: {}", index, peak)
					}
				},
				_ => break
			}
		}

		for (_, input) in self.sink_inputs.iter_mut() {
			let mut stream = input.monitor.borrow_mut();
			while stream.readable_size().is_some() {
				match stream.peek().unwrap() {
					PeekResult::Hole(_) => stream.discard().unwrap(),
					PeekResult::Data(b) => {
						let buf = slice_as_array!(b, [u8; 4]).expect("Bad length.");
						let peak = f32::from_le_bytes(*buf);
						input.peak = (peak * 150.0).round() as u32;
						stream.discard().unwrap();
					},
					_ => break
				}
			}
		}

		for (_, sink) in self.sinks.iter_mut() {
			let mut stream = sink.monitor.borrow_mut();
			while stream.readable_size().is_some() {
				match stream.peek().unwrap() {
					PeekResult::Hole(_) => stream.discard().unwrap(),
					PeekResult::Data(b) => {
						let buf = slice_as_array!(b, [u8; 4]).expect("Bad length.");
						let peak = f32::from_le_bytes(*buf);
						sink.peak = (peak * 150.0).round() as u32;
						stream.discard().unwrap();
					},
					_ => break
				}
			}
		}

		true
		// received
	}

	// pub fn cleanup(&mut self) {
	// 	let mut mainloop = self.mainloop.borrow_mut();
	// 	mainloop.unlock();
	// 	mainloop.stop();
	// }

	fn sink_updated(&mut self, data: SinkData) {
		let index = data.index;

		let entry = self.sinks.get_mut(&index);
		if entry.is_some() { entry.unwrap().data = data; }
		else {
			let stream = self.create_source_monitor(index, u32::MAX);
			self.sinks.insert(index, Sink {
				data, peak: 0, monitor: stream });
		}
	}

	fn sink_removed(&mut self, index: u32) {
		self.sinks.remove(&index);
	}

	fn sink_input_updated(&mut self, data: SinkInputData) {
		let index = data.index;
		let sink = data.sink;

		let entry = self.sink_inputs.get_mut(&index);
		if entry.is_some() { entry.unwrap().data = data; }
		else {
			let stream = self.create_source_monitor(sink, index);
			self.sink_inputs.insert(index, SinkInput {
				data, peak: 0, monitor: stream });
		}
	}

	fn sink_input_removed(&mut self, index: u32) {
		self.sink_inputs.remove(&index);
	}

	fn create_source_monitor(&mut self, _source_index: u32, stream_index: u32) -> Shared<Stream> {
		// TODO: Source index *must* be needed somewhere.
		// I think it's just happening to choose the right source when i input None below.
		
		let mut attr = BufferAttr::default();
		attr.fragsize = 4;
		attr.maxlength = u32::MAX;
		
		let spec = Spec { channels: 1, format: Format::F32le, rate: 25 };
		assert!(spec.is_valid());
		
		let s = Shared::new(Stream::new(&mut self.context.borrow_mut(), "VMix Peak Detect", &spec, None).unwrap());
		{
			let mut stream = s.borrow_mut();
			if stream_index != u32::MAX { stream.set_monitor_stream(stream_index).unwrap(); }

			stream.connect_record(None, Some(&attr),
				StreamFlagSet::DONT_MOVE | StreamFlagSet::ADJUST_LATENCY | StreamFlagSet::PEAK_DETECT).unwrap();

			let ss = s.clone();
			stream.set_state_callback(Some(Box::new(move || println!("{:?}", ss.borrow_mut().get_state()))));
		}

		return s;
	}
}

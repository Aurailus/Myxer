use std::collections::HashMap;

// use std::borrow::BorrowMut;
// use std::sync::{ Weak, Mutex };
use std::sync::mpsc::Sender;

use crate::shared::Shared;

use pulse::proplist::Proplist;
use pulse::callbacks::ListResult;
use pulse::mainloop::threaded::Mainloop;
use pulse::context::{ Context, FlagSet };
use pulse::context::introspect::{ SinkInputInfo, SinkInfo };
use pulse::context::subscribe::{ InterestMaskSet, Facility, Operation };

#[derive(Debug)]
pub enum PulseTx {
	INPUT(u32, Option<TxInput>),
	SINK(u32, Option<TxSink>),
	END()
}

#[derive(Debug)]
pub struct TxInput {
	name: String,
	icon: String
}

#[derive(Debug)]
pub struct TxSink {
	name: String,
	icon: String
}

#[derive(Debug)]
pub struct PulseStore {
	pub inputs: HashMap<u32, TxInput>
}

impl PulseStore {
	pub fn new() -> Self {
		PulseStore {
			// sink_inputs_dirty: false,
			inputs: HashMap::new()
		}
	}
}

pub struct PulseController {
	mainloop_shr: Shared<Mainloop>,
	context_shr: Shared<Context>,

	// sink_inputs: Shared<Vec<StoreSinkInfo>>,
	// pub sink_inputs_cb: Option<Box<dyn Fn(&Vec<StoreSinkInfo>) + 'callback>>
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

		PulseController {
			mainloop_shr: mainloop,
			context_shr: context,
			// sink_inputs: Shared::new(Vec::new()),
			// sink_inputs_cb: None
		}
	}

	pub fn cleanup(&mut self) {
		let mut ml = self.mainloop_shr.borrow_mut();
		ml.unlock();
		ml.stop();
	}

	pub fn connect(&mut self) {
		let mut ml = self.mainloop_shr.borrow_mut();
		let mut ctx = self.context_shr.borrow_mut();

		let ml_shr_ref = self.mainloop_shr.clone();
		let ctx_shr_ref = self.context_shr.clone();

		ctx.set_state_callback(Some(Box::new(move || {
			match unsafe { (*ctx_shr_ref.as_ptr()).get_state() } {
				pulse::context::State::Ready |
				pulse::context::State::Failed |
				pulse::context::State::Terminated => {
					unsafe { (*ml_shr_ref.as_ptr()).signal(false); }
				},
				_ => {},
			}
		})));

		ctx.connect(None, FlagSet::NOFLAGS, None)
			.expect("PulseController: Failed to connect the context to server.");

		ml.lock();
		ml.start().expect("PulseController: Failed to start mainloop.");

		loop {
			match ctx.get_state() {
				pulse::context::State::Ready => {
					ctx.set_state_callback(None);
					ml.unlock();
					break;
				},
				pulse::context::State::Failed |
				pulse::context::State::Terminated => {
					eprintln!("Context state failed/terminated, quitting...");
					ml.unlock();
					ml.stop();
					panic!("Pulse session terminated.");
				},
				_ => { ml.wait(); },
			}
		}
	}

	pub fn subscribe(&mut self, tx: std::sync::mpsc::Sender<PulseTx>) {
		let mut context = self.context_shr.borrow_mut();
		let introspect = context.introspect();

		fn sink_change(tx: Sender<PulseTx>, result: ListResult<&SinkInfo<'_>>) {
			match result {
				ListResult::Item(item) => {
					println!("{:?}", item);
					// tx.send(PulseTx::SINK(item.index, Some(TxInput {
					// 	name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
					// 	icon: item.proplist.get_str("application.icon").unwrap_or("".to_owned())
					// }))).unwrap();
				},
				_ => tx.send(PulseTx::END()).unwrap(),
			};
		};

		fn sink_input_change(tx: Sender<PulseTx>, result: ListResult<&SinkInputInfo<'_>>) {
			match result {
				ListResult::Item(item) => {
					tx.send(PulseTx::INPUT(item.index, Some(TxInput {
						name: item.proplist.get_str("application.name").unwrap_or("".to_owned()),
						icon: item.proplist.get_str("application.icon").unwrap_or("".to_owned())
					}))).unwrap();
				},
				_ => tx.send(PulseTx::END()).unwrap(),
			};
		};

		let txc = tx.clone();
		introspect.get_sink_input_info_list(move |res| sink_input_change(txc.clone(), res));
		
		context.subscribe(InterestMaskSet::SINK_INPUT | InterestMaskSet::SINK, |_|());

		context.set_subscribe_callback(Some(Box::new(move |fac, op, index| {
			let txc = tx.clone();
			let facility = fac.unwrap();
			let operation = op.unwrap();

			match facility {
				Facility::SinkInput => match operation {
					Operation::Removed => txc.send(PulseTx::INPUT(index, None)).unwrap(),
					_ => { introspect.get_sink_input_info(index, move |res| sink_input_change(txc.clone(), res)); }
				},
				Facility::Sink => match operation {
					Operation::Removed => txc.send(PulseTx::SINK(index, None)).unwrap(),
					_ => { introspect.get_sink_info_by_index(index, move |res| sink_change(txc.clone(), res)); }
				},
				_ => panic!("Subscribe callback received facility it didn't subscribe to: {:?}", facility)
			};
		})));
	}
}

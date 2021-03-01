use crate::shared::Shared;

use pulse::stream::{ Stream };

#[derive(Debug)]
pub struct SinkData {
	pub index: u32,
	pub name: String,
	pub description: String,
	pub port_description: String,
	pub monitor_index: u32,
	pub muted: bool,
	pub volume: u32
}

pub struct Sink {
	pub data: SinkData,

	pub peak: u32,
	pub monitor: Shared<Stream>
}

#[derive(Debug)]
#[derive(Clone)]
pub struct SinkInputData {
	pub index: u32,
	pub sink: u32,
	pub name: String,
	pub icon: String,
	pub muted: bool,
	pub volume: u32
}

pub struct SinkInput {
	pub data: SinkInputData,

	pub peak: u32,
	pub monitor: Shared<Stream>
}

#[derive(Debug)]
pub struct SourceData {
	pub index: u32,
	pub name: String,
	pub description: String,
	pub muted: bool,
	pub volume: u32,
}

pub struct Source {
	pub data: SourceData,

	pub peak: u32,
	pub monitor: Shared<Stream>
}

#[derive(Debug)]
#[derive(Clone)]
pub struct SourceOutputData {
	pub index: u32,
	pub source: u32,
	pub name: String,
	pub icon: String,
	pub muted: bool,
	pub volume: u32
}

pub struct SourceOutput {
	pub data: SourceOutputData,

	pub peak: u32,
	pub monitor: Shared<Stream>
}

<h1 align="center">Myxer</h1>

<p align="center">
  <img src="https://raw.githubusercontent.com/Aurailus/Myxer/master/media/myxer_dark.png">
</p>

<h3 align="center">A modern Volume Mixer for PulseAudio, built with you in mind.</h2>

<p align="center">
  <a href="https://github.com/Aurailus/Myxer/releases"><img src="https://github.com/Aurailus/Myxer/workflows/release/badge.svg" alt="Releases"/></a>
  <a href="https://aurail.us/discord"><img src="https://img.shields.io/discord/416379773976051712.svg?color=7289DA&label=discord&logo=discord&logoColor=white&labelColor=2A3037" alt="Join Discord"/></a>
  <a href="https://github.com/Aurailus/Myxer/commits/master"><img src="https://img.shields.io/github/commit-activity/m/aurailus/myxer.svg?logo=github&labelColor=2A3037&label=commit%20activity" alt="Commit Activity"/></a>
</p>

<br>

Myxer is a lightweight, powerful Volume Mixer built in Rust, that uses modern UI design to create a seamless user experience. Inputs, Outputs, Playback Streams, and Recording streams can all be managed with Myxer, giving you complete control over your system audio.

<br>
<br>

<img src="https://raw.githubusercontent.com/Aurailus/Myxer/master/media/myxer_light.png" align="left" width="625">

### Responsive

Myxer adapts to your selected GTK theme so that it fits seamlessly into your stock applications.

Additionally, PulseAudio plugin can be easily configured to open Myxer when you click the "Audio Mixer" entry on the popup menu, so it can behave like a stock app, too!

<br clear="left">
<br>
<br>

<img src="https://raw.githubusercontent.com/Aurailus/Myxer/master/media/myxer_advanced.png" align="right" width="650">

### Advanced

Behind the context menu you can find options to show individual audio channels, and even configure Audio Card profiles in the App. There's no need to pavucontrol anymore.

<br clear="right">
<br>
<br>

<h2 align="center">Open Source</h2>

Myxer is licensed permissively, under the [GNU Lesser Public License v3](https://github.com/Aurailus/Myxer/LICENSE.md). It's under active development, and all issues and pull requests will be responded to promptly. It's also super lightweight, and should only take an hour or two to read through the source code.

<br>
<br>

<h3 align="center">Heard enough? Download the <a href="https://github.com/Aurailus/Myxer/releases">Latest Release</a> here.</h3>

<p align="center"><em>Or, keep reading for alternative methods.</em></p>

<br>
<br>

### Building

Download the repository, and Cargo, and then simply run `cargo build --release` in the root directory.

Major releases are available on the [Releases](https://github.com/Aurailus/Myxer/releases) page. If you want something more breaking edge, you can download an artifact of the lastest commit [here](https://nightly.link/Aurailus/myxer/workflows/release/master/Myxer.zip). These artifacts are untested, YMMV.

#### Building for Development 

Call `cargo run`, (or `nodemon`, if you have that installed) in the root directory, and watch it go.

### Contributing
 
Pull Requests are welcome and appreciated. If you're unsure of whether a feature will be accepted, open an issue and I'll get back to you as soon as I can.  

<br>
<br>

&copy; [Auri Collings](https://twitter.com/Aurailus), 2021. Made with <3


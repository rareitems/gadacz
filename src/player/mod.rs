use std::path::Path;
use std::time::Duration;

use glib::BoolError;
use gst::event::Seek;
use gst::prelude::*;
use gstreamer as gst;

use crate::data::chapter::Chapter;

pub struct Player {
    pub playbin: gst::Element,
    pub state: Option<gst::State>,
    pub bus: gst::Bus,
    // pub uri: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    SendEventError,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SendEventError => write!(f, "SendEventError"),
        }
    }
}

impl Player {
    pub fn default() -> Self {
        // let playbin = gst::ElementFactory::make_with_name("playbin", Some("gadacz")).unwrap();
        let playbin = gst::ElementFactory::make("playbin").name("gadacz").build().unwrap();

        // let tempo = gst::ElementFactory::make_with_name("scaletempo", Some("tempo")).unwrap();
        let tempo = gst::ElementFactory::make("scaletempo").name("tempo").build().unwrap();

        // let sink = gst::ElementFactory::make_with_name("autoaudiosink",
        // Some("autoaudiosink")).unwrap();
        let sink = gst::ElementFactory::make("autoaudiosink").name("audiosink").build().unwrap();

        let bin = gst::Bin::new(Some("audiosink"));
        bin.add_many(&[&tempo, &sink]).unwrap();
        gst::Element::link_many(&[&tempo, &sink]).unwrap();
        tempo.sync_state_with_parent().unwrap();

        let pad = tempo.static_pad("sink").expect("Failed to get a static pad from equalizer.");

        let ghost_pad = gst::GhostPad::with_target(Some("sink"), &pad).unwrap();

        ghost_pad.set_active(true).unwrap();
        bin.add_pad(&ghost_pad).unwrap();
        playbin.set_property("audio-sink", &bin);

        let bus = playbin.bus().unwrap();

        Self {
            playbin,
            state: None,
            bus,
            // uri: None,
        }
    }

    pub fn get_volume(&mut self) -> f64 {
        self.playbin.property("volume")
    }

    /// Changes the state of the player to `Playing`. Will block if it hasn't happened immedietly
    pub fn play(&mut self) {
        let res = self.playbin.set_state(gst::State::Playing);

        self.wait_for_state_chage(gst::State::Playing, res).unwrap();
        self.state = Some(gst::State::Playing);
    }

    /// Changes the state of the player to `Paused`. Will block if it hasn't happened immedietly
    pub fn pause(&mut self) {
        let res = self.playbin.set_state(gst::State::Paused);

        self.wait_for_state_chage(gst::State::Paused, res).unwrap();
        self.state = Some(gst::State::Paused);
    }

    /// Changes the state of the player to `Null`. Will block if it hasn't happened immedietly
    pub fn null(&mut self) {
        let res = self.playbin.set_state(gst::State::Null);

        self.wait_for_state_chage(gst::State::Null, res).unwrap();
        self.state = Some(gst::State::Null);
    }

    /// Sets player's speed
    pub fn set_speed(&mut self, speed: f64) -> Result<(), Error> {
        let position = self
            .playbin
            .query_position::<gst::ClockTime>()
            .expect("Could not query current position.");

        let seek = Seek::new(
            speed,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            position,
            gst::SeekType::None,
            gst::ClockTime::ZERO,
        );

        match self.playbin.send_event(seek) {
            true => Ok(()),
            false => Err(Error::SendEventError),
        }
    }

    pub fn set_speed_and_position(
        &mut self,
        speed: f64,
        position: gst::ClockTime,
    ) -> Result<(), Error> {
        let seek = Seek::new(
            speed,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            position,
            gst::SeekType::None,
            gst::ClockTime::ZERO,
        );

        match self.playbin.send_event(seek) {
            true => Ok(()),
            false => Err(Error::SendEventError),
        }
    }

    /// Sets player's volume
    pub fn set_volume(&mut self, volume: f64) {
        self.playbin.set_property("volume", volume);
    }

    /// Changes player's volume by `change`
    pub fn change_volume(&mut self, change: f64) {
        let v = self.get_volume();
        self.set_volume(v + change);
    }

    /// Gets current position of thje player
    pub fn get_position(&self) -> Option<gstreamer::ClockTime> {
        self.playbin.query_position()
    }

    /// Gets current position of the player in seconds
    pub fn get_position_sec(&self) -> Option<u64> {
        self.playbin.query_position().map(gstreamer::ClockTime::seconds)
    }

    pub fn get_position_perc(&self) -> gstreamer::format::Percent {
        let position = self.playbin.query_position::<gstreamer::format::Percent>();
        match position {
            Some(ret) => ret,
            None => gstreamer::format::Percent::ZERO,
        }
    }

    pub fn get_total_duration(&self) -> gstreamer::ClockTime {
        self.playbin.query_duration().unwrap()
    }

    pub fn seek_seconds(&mut self, position: u64, speed: f64) -> Result<(), BoolError> {
        self.set_speed_and_position(speed, gst::ClockTime::SECOND * position).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    pub fn load_chapter(&mut self, chapter: &Chapter, path: &Path, speed: f64, volume: f64) {
        let mut path = path.to_path_buf();
        path.push(&chapter.filename);

        self.playbin.set_property("uri", crate::data::make_uri(&path));
        self.playbin.set_property("volume", volume);
        self.pause();

        let pos = if chapter.last_position != 0 {
            chapter.last_position
        } else {
            chapter.get_start_position()
        };
        self.set_speed_and_position(speed, gstreamer::ClockTime::from_seconds(pos)).unwrap();
    }

    pub fn wait_for_state_chage(
        &self,
        state: gst::State,
        res: Result<gst::StateChangeSuccess, gst::StateChangeError>,
    ) -> Result<(), gst::StateChangeError> {
        match res {
            Ok(ok) => match ok {
                gst::StateChangeSuccess::Success => return Ok(()),
                gst::StateChangeSuccess::Async => (),
                gst::StateChangeSuccess::NoPreroll => todo!(),
            },
            Err(err) => return Err(err),
        }

        for msg in self.bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    panic!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                }
                MessageView::StateChanged(state_changed) =>
                // We are only interested in state-changed messages from playbin
                {
                    if state_changed.src().map(|s| s == self.playbin).unwrap_or(false)
                        && state_changed.current() == state
                    {
                        break;
                    }
                }

                _ => (),
            }
        }

        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, Some(gst::State::Playing))
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.state, Some(gst::State::Paused))
    }

    pub fn if_playing_pause(&mut self) {
        if self.is_playing() {
            self.pause();
        }
    }

    /// Returns `true` if player is playing and pauses the playback, returns `false` otherwise
    pub fn is_playing_and_pause(&mut self) -> bool {
        if self.is_playing() {
            self.pause();
            true
        } else {
            false
        }
    }
}

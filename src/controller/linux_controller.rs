// Vkontroller - Turns your browser into a virtual game controller
// Copyright (C) 2026  flamfrosticboio
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::{
    controller::{ButtonMap, Controller, ControllerHandle, ControllerOutputEvent},
    server::ControllerInputEvent,
    shared::PlayerId,
};
use evdev::{
    AbsoluteAxisCode, AbsoluteAxisEvent, AttributeSet, EventSummary, FFEffectCode, FFStatusCode,
    InputEvent, InputId, KeyCode, KeyEvent, UInputCode,
    uinput::{VirtualDevice, VirtualEventStream},
};
use std::{fmt::Display, sync::Arc};
use tokio::select;

// this excludes the dpad since they have their different interfaces
const BUTTON_MAP: &[(ButtonMap, KeyCode)] = &[
    (ButtonMap::ButtonSouth, KeyCode::BTN_SOUTH),
    (ButtonMap::ButtonEast, KeyCode::BTN_EAST),
    // Holy shit why are they in reverse bro :cry:
    (ButtonMap::ButtonWest, KeyCode::BTN_NORTH),
    (ButtonMap::ButtonNorth, KeyCode::BTN_WEST),
    (ButtonMap::ButtonLeftBumper, KeyCode::BTN_TL),
    (ButtonMap::ButtonRightBumper, KeyCode::BTN_TR),
    (ButtonMap::ButtonSelect, KeyCode::BTN_SELECT),
    (ButtonMap::ButtonStart, KeyCode::BTN_START),
    (ButtonMap::ButtonGuide, KeyCode::BTN_MODE),
    (ButtonMap::ButtonLeftStick, KeyCode::BTN_THUMBL),
    (ButtonMap::ButtonRightStick, KeyCode::BTN_THUMBR),
];

enum StickId {
    Left,
    Right,
}

const MAX_EFFECTS: usize = 255;
type EffectId = i16;
pub struct FFEffectManager {
    counter: usize,
    free: Vec<EffectId>,
}

impl FFEffectManager {
    pub fn get_id(&mut self) -> Option<EffectId> {
        self.free.pop().or_else(|| {
            self.counter += 1;
            if self.counter < MAX_EFFECTS {
                // some problems
                match i16::try_from(self.counter) {
                    Ok(val) => Some(val),
                    Err(err) => {
                        tracing::error!(error = %err, "Could not convert id from usize to i16");
                        None
                    }
                }
            } else {
                None
            }
        })
    }

    pub fn return_id(&mut self, id: EffectId) {
        self.free.push(id);
    }
}

impl Default for FFEffectManager {
    fn default() -> Self {
        Self {
            counter: 0,
            free: Vec::with_capacity(MAX_EFFECTS),
        }
    }
}

pub struct LinuxController {
    player_id: PlayerId,
    event_stream: VirtualEventStream,
    handle: Arc<ControllerHandle>,
    input_rx: tokio::sync::mpsc::Receiver<ControllerInputEvent>,
    terminate_rx: tokio::sync::broadcast::Receiver<()>,
    effect_manager: tokio::sync::Mutex<FFEffectManager>,
}

impl LinuxController {
    // This part is only executed on creation of the controller
    // This is just split into function for readability
    #[inline]
    fn create_buttons() -> AttributeSet<evdev::KeyCode> {
        let mut keys = AttributeSet::<evdev::KeyCode>::new();
        for (_, keycode) in BUTTON_MAP.iter() {
            // this snippet below just copies alr
            keys.insert(*keycode)
        }

        keys
    }

    // This should be inline since its just a simple constructor
    #[inline]
    fn stick_abs() -> evdev::AbsInfo {
        evdev::AbsInfo::new(0, -32768, 32767, 16, 128, 1)
    }

    #[inline]
    fn hat_abs() -> evdev::AbsInfo {
        evdev::AbsInfo::new(0, -1, 1, 0, 0, 1)
    }

    #[inline]
    fn trigger_abs() -> evdev::AbsInfo {
        evdev::AbsInfo::new(0, 0, 255, 0, 0, 1)
    }

    async fn handle_event_item(&mut self, event: InputEvent) -> anyhow::Result<()> {
        // trusting that u16 can be transformed into i32
        const STOPPED: i32 = FFStatusCode::FF_STATUS_STOPPED.0 as i32;
        const PLAYING: i32 = FFStatusCode::FF_STATUS_PLAYING.0 as i32;

        match event.destructure() {
            EventSummary::ForceFeedback(.., _effect_id, STOPPED) => {
                tracing::debug!("Emitting effect(RumbleOff) on {}", self);
                self.handle
                    .send_output_update(ControllerOutputEvent::RumbleOff)
                    .await?;
            }

            EventSummary::ForceFeedback(.., _effect_id, PLAYING) => {
                tracing::debug!("Emitting effect(RumbleOn) on {}", self);
                self.handle
                    .send_output_update(ControllerOutputEvent::RumbleOn)
                    .await?;
            }
            EventSummary::UInput(event, UInputCode::UI_FF_UPLOAD, ..) => {
                let id = {
                    let mut manager = self.effect_manager.lock().await;
                    manager.get_id()
                };

                // CAUTION: This approach is unsafe since it will lead to thread deadlocks
                let device = self.event_stream.device_mut();
                let mut event = device.process_ff_upload(event)?;

                match id {
                    Some(id) => {
                        event.set_effect_id(id);
                        event.set_retval(0);
                        tracing::debug!("FF upload complete with id {} for {}", id, self);
                    }
                    None => {
                        tracing::error!("Failed to upload ffevent for {}", self);
                        event.set_retval(-1);
                    }
                }
            }
            EventSummary::UInput(event, UInputCode::UI_FF_ERASE, ..) => {
                let mut manager = self.effect_manager.lock().await;

                // CAUTION: This approach is unsafe since it will lead to thread deadlocks
                let device = self.event_stream.device_mut();
                let event = device.process_ff_erase(event)?;
                manager.return_id(event.effect_id() as i16);
            }
            _ => {}
        };

        Ok(())
    }

    fn construct_event_button(button: ButtonMap) -> Vec<InputEvent> {
        let mut res = Vec::with_capacity(BUTTON_MAP.len() + 2);
        let x = (button.contains(ButtonMap::DpadRight) as i32)
            - (button.contains(ButtonMap::DpadLeft) as i32);
        let y = (button.contains(ButtonMap::DpadDown) as i32)
            - (button.contains(ButtonMap::DpadUp) as i32);

        for (current_button, keycode) in BUTTON_MAP.iter() {
            let event = KeyEvent::new(*keycode, button.contains(*current_button) as i32);
            res.push(*event);
        }

        res.push(*AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_HAT0X, x));
        res.push(*AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_HAT0Y, y));

        res
    }

    fn construct_event_trigger(left: u8, right: u8) -> Vec<InputEvent> {
        vec![
            *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_Z, i32::from(left)),
            *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_RZ, i32::from(right)),
        ]
    }

    fn construct_event_stick(kind: StickId, x: i16, y: i16) -> Vec<InputEvent> {
        let (x_code, y_code) = match kind {
            StickId::Left => (
                evdev::AbsoluteAxisCode::ABS_X,
                evdev::AbsoluteAxisCode::ABS_Y,
            ),
            StickId::Right => (
                evdev::AbsoluteAxisCode::ABS_RX,
                evdev::AbsoluteAxisCode::ABS_RY,
            ),
        };

        vec![
            *evdev::AbsoluteAxisEvent::new(x_code, i32::from(x)),
            *evdev::AbsoluteAxisEvent::new(y_code, i32::from(y)),
        ]
    }
}

impl Display for LinuxController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Controller(id: {})", self.player_id)
    }
}

impl Controller for LinuxController {
    fn new(
        player_id: PlayerId,
        input_rx: tokio::sync::mpsc::Receiver<ControllerInputEvent>,
        terminate_rx: tokio::sync::broadcast::Receiver<()>,
        handle: Arc<ControllerHandle>,
    ) -> anyhow::Result<Box<Self>> {
        // Don't worry, the compiler will just optimize this part
        let device_name: &str = "Microsoft Xbox One S Controller";
        let device_id: InputId = InputId::new(evdev::BusType::BUS_USB, 0x045e, 0x02ea, 0x0408);

        let buttons = Self::create_buttons();
        let device = VirtualDevice::builder()?
            .name(device_name)
            .input_id(device_id)
            .with_keys(&buttons)?
            // Left stick
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_X,
                Self::stick_abs(),
            ))?
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Y,
                Self::stick_abs(),
            ))?
            // Right stick
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_RX,
                Self::stick_abs(),
            ))?
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_RY,
                Self::stick_abs(),
            ))?
            // Analog triggers
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Z,
                Self::trigger_abs(),
            ))?
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_RZ,
                Self::trigger_abs(),
            ))?
            // D-pad
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_HAT0X,
                Self::hat_abs(),
            ))?
            .with_absolute_axis(&evdev::UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_HAT0Y,
                Self::hat_abs(),
            ))?
            .with_ff(&AttributeSet::from_iter([FFEffectCode::FF_RUMBLE]))?
            // very questionable config, but you know what lets set it to this value
            .with_ff_effects_max(16)
            .build()?;

        let event_stream = device.into_event_stream()?;

        Ok(Box::new(Self {
            player_id,
            event_stream,
            handle,
            input_rx,
            terminate_rx,
            effect_manager: tokio::sync::Mutex::new(FFEffectManager::default()),
        }))
    }

    async fn run_event(mut self: Box<Self>) -> anyhow::Result<()> {
        let self_id = self.to_string();

        'event_loop: loop {
            select! {
                _ = self.terminate_rx.recv() => {
                    tracing::debug!("Received termination event for {}", self_id);
                    break 'event_loop;
                }
                event_result = self.event_stream.next_event() => {
                    let event = event_result?;
                    Self::handle_event_item(&mut self, event).await?;
                }
                item = self.input_rx.recv() => {
                    if let Some(item) = item {

                        let event = match item {
                            ControllerInputEvent::Button(buttons) => Self::construct_event_button(ButtonMap::from_bits_retain(buttons)),
                            ControllerInputEvent::Triggers(left, right) => Self::construct_event_trigger(left, right),
                            ControllerInputEvent::StickLeft(x, y) => Self::construct_event_stick(StickId::Left, x, y),
                            ControllerInputEvent::StickRight(x, y) => Self::construct_event_stick(StickId::Right, x, y),
                        };

                        tracing::debug!("Attempting to send kernel commands for {} with {:#?}", self_id, event);

                        let device = &mut self.event_stream.device_mut();
                        device.emit(event.as_slice())?;

                        tracing::debug!("Successfully sent commands to kernel for {}", self_id);
                    }
                }
            }
        }

        tracing::debug!(
            "{} controller main event loop successfully stopped",
            self_id
        );

        Ok(())
    }
}

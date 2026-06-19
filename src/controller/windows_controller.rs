use std::{fmt::Display, sync::Arc, thread};

use anyhow::Context;
use vigem_rust::{Client, X360Button, X360Notification, X360Report};

use crate::{
    controller::{ButtonMap, Controller, ControllerHandle, ControllerOutputEvent},
    server::ControllerInputEvent::{self, Button, StickLeft, StickRight, Triggers},
    shared::PlayerId,
};

pub struct WindowsController {
    id: PlayerId,
    input_rx: tokio::sync::mpsc::Receiver<ControllerInputEvent>,
    terminate_rx: tokio::sync::broadcast::Receiver<()>,
    handle: Arc<ControllerHandle>,
}

impl Display for WindowsController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Controller(id: {})", self.id)
    }
}

impl WindowsController {
    fn map_buttons(input: ButtonMap) -> X360Button {
        let mut raw_xbuttons: X360Button = X360Button::empty();

        // D-pad buttons
        if input.contains(ButtonMap::DpadUp) {
            raw_xbuttons |= X360Button::DPAD_UP;
        }
        if input.contains(ButtonMap::DpadDown) {
            raw_xbuttons |= X360Button::DPAD_DOWN;
        }
        if input.contains(ButtonMap::DpadLeft) {
            raw_xbuttons |= X360Button::DPAD_LEFT;
        }
        if input.contains(ButtonMap::DpadRight) {
            raw_xbuttons |= X360Button::DPAD_RIGHT;
        }

        // Face buttons
        if input.contains(ButtonMap::ButtonSouth) {
            raw_xbuttons |= X360Button::A;
        }
        if input.contains(ButtonMap::ButtonEast) {
            raw_xbuttons |= X360Button::B;
        }
        if input.contains(ButtonMap::ButtonWest) {
            raw_xbuttons |= X360Button::X;
        }
        if input.contains(ButtonMap::ButtonNorth) {
            raw_xbuttons |= X360Button::Y;
        }

        // Bumpers and sticks
        if input.contains(ButtonMap::ButtonLeftBumper) {
            raw_xbuttons |= X360Button::LEFT_SHOULDER;
        }
        if input.contains(ButtonMap::ButtonRightBumper) {
            raw_xbuttons |= X360Button::RIGHT_SHOULDER;
        }
        if input.contains(ButtonMap::ButtonLeftStick) {
            raw_xbuttons |= X360Button::LEFT_THUMB;
        }
        if input.contains(ButtonMap::ButtonRightStick) {
            raw_xbuttons |= X360Button::RIGHT_THUMB;
        }

        // Special buttons
        if input.contains(ButtonMap::ButtonStart) {
            raw_xbuttons |= X360Button::START;
        }
        if input.contains(ButtonMap::ButtonSelect) {
            raw_xbuttons |= X360Button::BACK;
        }
        if input.contains(ButtonMap::ButtonGuide) {
            raw_xbuttons |= X360Button::GUIDE;
        }

        raw_xbuttons
    }

    fn handle_event(event: ControllerInputEvent, buffer: &mut X360Report) {
        match event {
            Button(button_raw) => {
                buffer.buttons = Self::map_buttons(ButtonMap::from_bits_retain(button_raw));
            }
            Triggers(left, right) => {
                buffer.left_trigger = left;
                buffer.right_trigger = right;
            }

            StickLeft(x, y) => {
                buffer.thumb_lx = x;
                buffer.thumb_ly = -y;
            }

            StickRight(x, y) => {
                buffer.thumb_rx = x;
                buffer.thumb_ry = -y;
            }
        }
    }

    fn handle_notification(
        notification: X360Notification,
        handle: &Arc<ControllerHandle>,
    ) -> anyhow::Result<()> {
        tracing::info!("notification: {:?}", notification);
        handle.output_channel.blocking_send(
            crate::controller::ControllerOutputEvent::PlayerChange(notification.led_number as u32),
        )?;

        if notification.large_motor > 0 || notification.small_motor > 0 {
            handle
                .output_channel
                .blocking_send(ControllerOutputEvent::RumbleOn)?;
        } else {
            handle
                .output_channel
                .blocking_send(ControllerOutputEvent::RumbleOff)?;
        }

        Ok(())
    }

    fn spawn_blocking(&mut self) -> anyhow::Result<()> {
        let client = Client::connect()?;
        let target = client.new_x360_target().plugin()?;

        // wait for the controller to load
        target.wait_for_ready()?;

        // Sender par
        let notification_receiver = target.register_notification()?;
        let handle = self.handle.clone();
        thread::spawn(move || {
            while let Ok(Ok(notification)) = notification_receiver.recv() {
                if let Err(err) = Self::handle_notification(notification, &handle) {
                    tracing::error!(err=%err, "Could not run notification handler");
                    break;
                };
            }
        });

        let self_string = self.to_string();

        // Receiver part
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        // If there are blocking calls, it would be neglectable since this runtime
        // is already on its own separate thread
        let result: anyhow::Result<()> = runtime.block_on(async move {
            let mut buffer = vigem_rust::X360Report::default();
            loop {
                tokio::select! {
                    _ = self.terminate_rx.recv() => { break Ok(()); }
                    Some(event) = self.input_rx.recv() => {
                        Self::handle_event(event, &mut buffer);
                        target.update(&buffer).context("Could not update controller")?;
                    }
                }
            }
        });

        tracing::info!("{}'s runner has stopped", self_string);

        result
    }
}

impl Controller for WindowsController {
    fn new(
        id: crate::shared::PlayerId,
        input_rx: tokio::sync::mpsc::Receiver<ControllerInputEvent>,
        terminate_rx: tokio::sync::broadcast::Receiver<()>,
        handle: Arc<ControllerHandle>,
    ) -> anyhow::Result<Box<Self>> {
        Ok(Box::new(Self {
            id,
            input_rx,
            terminate_rx,
            handle,
        }))
    }

    async fn run_event(mut self: Box<Self>) -> anyhow::Result<()> {
        // let mut stopper = self.terminate_rx.resubscribe();

        thread::spawn(move || {
            match self.spawn_blocking() {
                Ok(_) => {}
                Err(err) => {
                    tracing::error!(error = %err, "Could not run controller");
                    // if let Err(err) = self
                    //     .handle
                    //     .terminate()
                    //     .context("Could not safely terminate during error")
                    // {
                    //     tracing::error!(err=%err, "Error during error termination")
                    // }
                }
            };
        });

        // this will async block to prevent it to return quickly
        // stopper.recv().await?;

        Ok(())
    }
}

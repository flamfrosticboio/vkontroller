use crate::{server::ControllerInputEvent, shared::PlayerId};
use anyhow::Context;
use std::{fmt::Display, sync::Arc};

#[cfg(target_os = "linux")]
pub mod linux_controller;

pub enum StickId {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub enum ControllerOutputEvent {
    RumbleOn,
    RumbleOff,
    PlayerChange(u32),
}

impl ControllerOutputEvent {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::RumbleOn => vec![0x01],
            Self::RumbleOff => vec![0x02],
            Self::PlayerChange(player) => {
                let mut vecs = player.to_le_bytes().to_vec();
                vecs.push(0x01); // the type
                return vecs;
            }
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ButtonMap: u32 {
        const DpadUp = 1 << 0;
        const DpadDown = 1 << 1;
        const DpadLeft = 1 << 2;
        const DpadRight = 1 << 3;

        const ButtonWest = 1 << 4;
        const ButtonNorth = 1 << 5;
        const ButtonSouth = 1 << 6;
        const ButtonEast = 1 << 7;

        const ButtonLeftBumper = 1 << 8;
        const ButtonRightBumper = 1 << 9;
        const ButtonLeftStick = 1 << 10; // aka. L3
        const ButtonRightStick = 1 << 11; // aka. R3

        const ButtonStart = 1 << 12;
        const ButtonSelect = 1 << 13;
        const ButtonGuide = 1 << 14;
    }
}

pub struct ControllerHandle {
    terminate_signal: tokio::sync::broadcast::Sender<()>,
    input_channel: tokio::sync::mpsc::Sender<ControllerInputEvent>,
    output_channel: tokio::sync::mpsc::Sender<ControllerOutputEvent>,
    id: PlayerId,
}

impl Display for ControllerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ControllerHandle({})", self.id)
    }
}

impl ControllerHandle {
    pub fn new(
        id: PlayerId,
        terminate_signal: tokio::sync::broadcast::Sender<()>,
        input_channel: tokio::sync::mpsc::Sender<ControllerInputEvent>,
        output_channel: tokio::sync::mpsc::Sender<ControllerOutputEvent>,
    ) -> Self {
        return Self {
            id: id,
            terminate_signal: terminate_signal,
            input_channel: input_channel,
            output_channel: output_channel,
        };
    }

    pub async fn send_input_update(&self, value: ControllerInputEvent) -> anyhow::Result<()> {
        self.input_channel
            .send(value.clone())
            .await
            .with_context(|| {
                format!(
                    "[{}][Interface]: Failed to send input update with value {:?}",
                    self, value
                )
            })
    }

    pub async fn send_output_update(&self, value: ControllerOutputEvent) -> anyhow::Result<()> {
        self.output_channel
            .send(value.clone())
            .await
            .with_context(|| {
                format!(
                    "[{}][Interface]: Failed to send output update with value {:?}",
                    self, value
                )
            })
    }

    pub fn terminate(&self) -> anyhow::Result<()> {
        let receivers = self.terminate_signal.send(())?;
        if receivers != 1 {
            return Err(anyhow::format_err!(
                "Termination for {} was sent, but there are {} receivers (expected: 1). Report this as a bug.",
                self,
                receivers
            ));
        }
        Ok(())
    }
}

pub trait Controller: Display {
    // The controller is placed on Box (or heap) since it would be dynamically allocated
    fn new(
        id: PlayerId,
        output_channel_tx: tokio::sync::mpsc::Receiver<ControllerInputEvent>,
        terminate_signal: tokio::sync::broadcast::Receiver<()>,
        handle: Arc<ControllerHandle>,
    ) -> anyhow::Result<Box<Self>>;
    fn run_event(self: Box<Self>) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub fn create_controller(
    id: PlayerId,
    output_channel_tx: tokio::sync::mpsc::Sender<ControllerOutputEvent>,
) -> anyhow::Result<(Box<impl Controller>, Arc<ControllerHandle>)> {
    let (input_channel_tx, input_channel_rx) =
        tokio::sync::mpsc::channel::<ControllerInputEvent>(1024);

    let (terminate_tx, terminate_rx) = tokio::sync::broadcast::channel(32);
    let handle = Arc::new(ControllerHandle::new(
        id,
        terminate_tx,
        input_channel_tx,
        output_channel_tx,
    ));

    #[cfg(target_os = "linux")]
    {
        let controller = linux_controller::LinuxController::new(
            id,
            input_channel_rx,
            terminate_rx,
            handle.clone(),
        )?;
        return Ok((controller, handle.clone()));
    }
    // just write manually for windows atp
}
